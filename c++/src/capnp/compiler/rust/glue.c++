// glue.c++

#include "glue.h"
#include <capnp/compiler/compiler.h>
#include <capnp/compiler/parser.h>
#include <capnp/compiler/module-loader.h>
#include <capnp/serialize.h>
#include <kj/map.h>
#include <utility>

using namespace capnp;
using namespace compiler;

class ErrorImpl : public kj::Exception, public std::exception {
public:
  inline ErrorImpl(Exception&& other) : Exception(kj::mv(other)) { }
  inline ErrorImpl(const ErrorImpl& self) : Exception(self) { }

  const char* what() const noexcept override {
    whatBuffer = kj::str(*this);
    return whatBuffer.begin();
  }

private:
  mutable kj::String whatBuffer;
};

class Glue final : public GlobalErrorReporter {
  ModuleLoader loader;
  Compiler compiler;
  kj::Own<kj::Filesystem> disk;
  kj::HashMap<kj::Path, std::pair<kj::Own<const kj::ReadableDirectory>, bool>> sourceDirectories;
  kj::HashMap<const kj::ReadableDirectory*, kj::String> dirPrefixes;
  bool addStandardImportPaths;
  uint compileEagerness = Compiler::NODE | Compiler::CHILDREN |
    Compiler::DEPENDENCIES | Compiler::DEPENDENCY_PARENTS;
  kj::Vector<kj::String> errors;

  struct SourceFile {
    uint64_t id;
    Compiler::ModuleScope compiled;
    kj::StringPtr name;
    Module* module;
  };

  kj::Vector<SourceFile> sourceFiles;

  virtual void addError(const kj::ReadableDirectory& directory, kj::PathPtr path,
    SourcePos start, SourcePos end,
    kj::StringPtr message) {
    errors.add(kj::str(path.toString(false).asPtr(), " [", start.line, ":", start.column, " - ", end.line, ":", end.column, "] ", message));
  }

  virtual bool hadErrors() { return !errors.empty(); }

public:
  Glue(bool addStandardImports) : disk(kj::newDiskFilesystem()), loader(*this), addStandardImportPaths(addStandardImports) {}

  void throwErrors(const char* file, int line) {
    throw ErrorImpl(kj::Exception(kj::Exception::Type::FAILED, file, line, kj::strArray(errors, "\n")));
  }

  std::pair<const kj::ReadableDirectory&, kj::Path> interpretSourceFile(kj::StringPtr pathStr) {
    auto cwd = disk->getCurrentPath();
    auto path = cwd.evalNative(pathStr);

    KJ_REQUIRE(path.size() > 0);
    for (size_t i = path.size() - 1; i > 0; i--) {
      auto prefix = path.slice(0, i);
      auto remainder = path.slice(i, path.size());

      KJ_IF_SOME(sdir, sourceDirectories.find(prefix)) {
        if (sdir.second) {
          return { *sdir.first, remainder.clone() };
        }
      }
    }

    // No source prefix matched. Fall back to heuristic: try stripping the current directory,
    // otherwise don't strip anything.
    if (path.startsWith(cwd)) {
      return { disk->getCurrent(), path.slice(cwd.size(), path.size()).clone() };
    }
    else {
      return { disk->getRoot(), kj::mv(path) };
    }
  }

  kj::Maybe<const kj::ReadableDirectory&> getSourceDirectory(kj::StringPtr pathStr, bool isSourcePrefix) {
    auto cwd = disk->getCurrentPath();
    auto path = cwd.evalNative(pathStr);

    if (path.size() == 0) return disk->getRoot();

    KJ_IF_SOME(sdir, sourceDirectories.find(path)) {
      sdir.second = sdir.second || isSourcePrefix;
      return *sdir.first;
    }

    if (path == cwd) {
      auto& result = disk->getCurrent();
      if (isSourcePrefix) {
        kj::Own<const kj::ReadableDirectory> fakeOwn(&result, kj::NullDisposer::instance);
        sourceDirectories.insert(kj::mv(path), { kj::mv(fakeOwn), isSourcePrefix });
      }
      return result;
    }

    KJ_IF_SOME(dir, disk->getRoot().tryOpenSubdir(path)) {
      auto& result = *dir.get();
      sourceDirectories.insert(kj::mv(path), { kj::mv(dir), isSourcePrefix });
#if _WIN32
      kj::String prefix = pathStr.endsWith("/") || pathStr.endsWith("\\")
        ? kj::str(pathStr) : kj::str(pathStr, '\\');
#else
      kj::String prefix = pathStr.endsWith("/") ? kj::str(pathStr) : kj::str(pathStr, '/');
#endif
      dirPrefixes.insert(&result, kj::mv(prefix));
      return result;
    }
    else {
      return kj::none;
    }
  }

  bool addImportPath(kj::StringPtr path) {
    KJ_IF_SOME(dir, getSourceDirectory(path, false)) {
      loader.addImportPath(dir);
      return true;
    }
    return false;
  }

  void addSource(kj::StringPtr file) {
    if (addStandardImportPaths) {
      static constexpr kj::StringPtr STANDARD_IMPORT_PATHS[] = {
        "/usr/local/include"_kj,
        "/usr/include"_kj,
#ifdef CAPNP_INCLUDE_DIR
        KJ_CONCAT(CAPNP_INCLUDE_DIR, _kj),
#endif
      };
      for (auto path : STANDARD_IMPORT_PATHS) {
        KJ_IF_SOME(dir, getSourceDirectory(path, false)) {
          loader.addImportPath(dir);
        }
    else {
      // ignore standard path that doesn't exist
    }
      }

      addStandardImportPaths = false;
    }

    auto dirPathPair = interpretSourceFile(file);
    KJ_IF_SOME(module, loader.loadModule(dirPathPair.first, dirPathPair.second)) {
      auto compiled = compiler.add(module);
      compiler.eagerlyCompile(compiled.getId(), compileEagerness);
      sourceFiles.add(SourceFile{ compiled.getId(), compiled, module.getSourceName(), &module });
    }
    else {
      KJ_FAIL_REQUIRE("Can't find source file!");
    }
  }

  kj::Maybe<kj::Array<capnp::word>> generateOutput() {
    if (hadErrors()) {
      // Skip output if we had any errors.
      return kj::none;
    }

    // We require one or more sources and if they failed to compile we quit above, so this should
    // pass.  (This assertion also guarantees that `compiler` has been initialized.)
    KJ_ASSERT(sourceFiles.size() > 0, "Shouldn't have gotten here without sources.");

    MallocMessageBuilder message;
    auto request = message.initRoot<schema::CodeGeneratorRequest>();

    auto version = request.getCapnpVersion();
    version.setMajor(CAPNP_VERSION_MAJOR);
    version.setMinor(CAPNP_VERSION_MINOR);
    version.setMicro(CAPNP_VERSION_MICRO);

    auto schemas = compiler.getLoader().getAllLoaded();
    auto nodes = request.initNodes(schemas.size());
    for (size_t i = 0; i < schemas.size(); i++) {
      nodes.setWithCaveats(i, schemas[i].getProto());
    }

    request.adoptSourceInfo(compiler.getAllSourceInfo(message.getOrphanage()));

    auto requestedFiles = request.initRequestedFiles(sourceFiles.size());
    for (size_t i = 0; i < sourceFiles.size(); i++) {
      auto requestedFile = requestedFiles[i];
      requestedFile.setId(sourceFiles[i].id);
      requestedFile.setFilename(sourceFiles[i].name);
      requestedFile.adoptImports(compiler.getFileImportTable(
        *sourceFiles[i].module, Orphanage::getForMessageContaining(requestedFile)));
    }

    return messageToFlatArray(message);
  }
};

rust::Vec<uint8_t> command(rust::Slice<const rust::String> files,
  rust::Slice<const rust::String> imports,
  rust::Slice<const rust::String> prefixes,
  bool standard_import)
{
  Glue glue(standard_import);

  for (auto prefix : prefixes) {
    if (glue.getSourceDirectory(prefix.c_str(), true) == kj::none) {
      KJ_FAIL_REQUIRE("No such directory.");
    }
  }

  for (auto imp : imports) {
    if (!glue.addImportPath(imp.c_str())) {
      KJ_FAIL_REQUIRE("Import path does not exist.");
    }
  }

  for (auto file : files) {
    glue.addSource(file.c_str());
  }

  KJ_IF_SOME(out, glue.generateOutput()) {
    auto bytes = out.asBytes();
    rust::Vec<uint8_t> result;
    result.reserve(bytes.size());
    std::move(bytes.begin(), bytes.end(), std::back_inserter(result));
    return result;
  }

  glue.throwErrors(__FILE__, __LINE__);
  KJ_UNREACHABLE;
}

uint64_t genRandId() {
  return generateRandomId();
}
