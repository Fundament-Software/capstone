use eyre::eyre;
use eyre::Result;
use std::{env, path::Path};

const CAPNP_HEAVY: bool = cfg!(feature = "heavy");
const KJ_ASYNC: bool = cfg!(feature = "async");
const KJ_HTTP: bool = cfg!(feature = "http");
const KJ_WITH_SSL: bool = cfg!(feature = "ssl");
const KJ_WITH_ZLIB: bool = cfg!(feature = "zlib");

static KJ_SOURCES_LITE: &[&str] = &[
    "arena.c++",
    "array.c++",
    "cidr.c++",
    "common.c++",
    "debug.c++",
    "encoding.c++",
    "exception.c++",
    "glob-filter.c++",
    "hash.c++",
    "io.c++",
    "list.c++",
    "main.c++",
    "memory.c++",
    "mutex.c++",
    "source-location.c++",
    "string.c++",
    "table.c++",
    "test-helpers.c++",
    "thread.c++",
    "units.c++",
    "time.c++", // capnproto incorrectly relies on this in mutex.c++ on windows only.
];
static KJ_SOURCES_HEAVY: &[&str] = &[
    "filesystem.c++",
    "filesystem-disk-unix.c++",
    "filesystem-disk-win32.c++",
    "parse/char.c++",
    "refcount.c++",
    "string-tree.c++",
];
static KJ_HEADERS: &[&str] = &["arena.h", "common.h", "list.h"];
static KJ_PRIVATE_HEADERS: &[&str] = &[
    "array.h",
    "cidr.h",
    "debug.h",
    "encoding.h",
    "exception.h",
    "filesystem.h",
    "function.h",
    "glob-filter.h",
    "hash.h",
    "io.h",
    "main.h",
    "map.h",
    "memory.h",
    "miniposix.h",
    "mutex.h",
    "one-of.h",
    "refcount.h",
    "source-location.h",
    "string.h",
    "string-tree.h",
    "table.h",
    "test.h",
    "thread.h",
    "time.h",
    "tuple.h",
    "units.h",
    "vector.h",
    "win32-api-version.h",
    "windows-sanity.h",
];
static KJ_PARSE_HEADERS: &[&str] = &["parse/common.h", "parse/char.h"];
static KJ_STD_HEADERS: &[&str] = &["std/iostream.h"];

static KJ_ASYNC_SOURCES: &[&str] = &[
    "async.c++",
    "async-unix.c++",
    "async-win32.c++",
    "async-io-win32.c++",
    "async-io.c++",
    "async-io-unix.c++",
    "timer.c++",
];
static KJ_ASYNC_HEADERS: &[&str] = &[
    "async-prelude.h",
    "async.h",
    "async-inl.h",
    "async-unix.h",
    "async-win32.h",
    "async-io.h",
    "async-queue.h",
    "timer.h",
];

static KJ_HTTP_SOURCES: &[&str] = &["compat/http.c++", "compat/url.c++"];
static KJ_HTTP_HEADERS: &[&str] = &["compat/http.h", "compat/url.h"];

static KJ_TLS_SOURCES: &[&str] = &["compat/readiness-io.c++", "compat/tls.c++"];
static KJ_TLS_HEADERS: &[&str] = &["compat/readiness-io.h", "compat/tls.h"];

static KJ_GZIP_SOURCES: &[&str] = &["compat/gzip.c++"];
static KJ_GZIP_HEADERS: &[&str] = &["compat/gzip.h"];

fn get_predicate(predicate: bool, src: &'static [&'static str]) -> &'static [&'static str] {
    if predicate {
        src
    } else {
        &[]
    }
}
static KJ_ASYNC_PRIVATE_HEADERS: &[&str] = &["async-io-internal.h", "miniposix.h"];

fn main() -> Result<()> {
    let out_dir = env::var_os("OUT_DIR").ok_or_else(|| eyre!("OUT_DIR not set"))?;
    let source_dir = Path::new(&out_dir)
        .join("cxxbridge")
        .join("crate")
        .join("kj");

    cxx_build::CFG.include_prefix = "kj";
    let mut build = cxx_build::bridge("lib.rs");

    KJ_HEADERS
        .iter()
        .chain(KJ_PARSE_HEADERS)
        .chain(KJ_STD_HEADERS)
        .chain(KJ_PRIVATE_HEADERS)
        .chain(get_predicate(KJ_ASYNC, KJ_ASYNC_HEADERS))
        .chain(get_predicate(KJ_ASYNC, KJ_ASYNC_PRIVATE_HEADERS))
        .chain(get_predicate(KJ_HTTP, KJ_HTTP_HEADERS))
        .chain(get_predicate(KJ_WITH_SSL, KJ_TLS_HEADERS))
        .chain(get_predicate(KJ_WITH_ZLIB, KJ_GZIP_HEADERS))
        .for_each(|s| println!("cargo:rerun-if-changed={}", s));

    // kj-async files break capnproto's own import conventions and are impossible to compile
    // seperately without significant header changes, so we compile it into the library as a feature.
    KJ_SOURCES_LITE
        .iter()
        .chain(get_predicate(KJ_ASYNC, KJ_ASYNC_SOURCES))
        .chain(get_predicate(CAPNP_HEAVY, KJ_SOURCES_HEAVY))
        .chain(get_predicate(KJ_HTTP, KJ_HTTP_SOURCES))
        .chain(get_predicate(KJ_WITH_SSL, KJ_TLS_SOURCES))
        .chain(get_predicate(KJ_WITH_ZLIB, KJ_GZIP_SOURCES))
        .map(|s| (s, source_dir.join(s)))
        .for_each(|(s, p)| {
            println!("cargo:rerun-if-changed={}", s);
            // This copy is only here in case the symlink fails on windows
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            let _ = match std::fs::exists(&p) {
                Ok(true) => Ok(0),
                _ => std::fs::copy(Path::new(s), &p),
            };
            build.file(p);
        });

    if !CAPNP_HEAVY {
        build.define("CAPNP_LITE", "1");
    }

    #[cfg(not(target_os = "windows"))]
    println!("cargo:rustc-link-lib=pthread");
    if cfg!(feature = "libdl") {
        build.define("KJ_HAS_LIBDL", None);
        println!("cargo:rustc-link-lib=dl");
    }

    // Unfuck MSVC
    build.flag_if_supported("/Zc:__cplusplus");
    build.flag_if_supported("/EHsc");
    build.flag_if_supported("/TP");
    build.opt_level(3);
    build.warnings(false).std("c++20").compile("kj");

    Ok(())
}
