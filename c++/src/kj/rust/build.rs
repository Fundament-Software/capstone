use eyre::eyre;
use eyre::Result;
use kj_build::kj_configure;
use kj_build::stage_files;
use std::{env, fs, path::Path};

const CAPNP_HEAVY: bool = cfg!(feature = "heavy");

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
];
static KJ_SOURCES_HEAVY: &[&str] = &[
    "filesystem.c++",
    "filesystem-disk-unix.c++",
    "filesystem-disk-win32.c++",
    "parse/char.c++",
    "refcount.c++",
    "string-tree.c++",
    "time.c++",
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
static KJ_ASYNC_PRIVATE_HEADERS: &[&str] = &["async-io-internal.h", "miniposix.h"];

fn main() -> Result<()> {
    let out_dir = env::var_os("OUT_DIR").ok_or_else(|| eyre!("OUT_DIR not set"))?;
    let sources = Path::new(&out_dir).join("sources");
    let source_dir = Path::new("..");
    let kj_source_dir = sources.join("kj");

    //let _ = fs::remove_dir_all(&kj_source_dir);
    fs::create_dir_all(&kj_source_dir)?;
    fs::create_dir_all(kj_source_dir.join("parse"))?;
    fs::create_dir_all(kj_source_dir.join("std"))?;

    cxx_build::CFG.exported_header_dirs.push(&sources);
    cxx_build::CFG.include_prefix = "kj";
    let mut build = cxx_build::bridge("lib.rs");

    stage_files(
        &mut build,
        KJ_HEADERS
            .iter()
            .chain(KJ_PARSE_HEADERS)
            .chain(KJ_STD_HEADERS)
            .chain(KJ_PRIVATE_HEADERS)
            .chain(KJ_ASYNC_HEADERS)
            .chain(KJ_ASYNC_PRIVATE_HEADERS),
        source_dir,
        &kj_source_dir,
        false,
    )?;

    stage_files(
        &mut build,
        KJ_SOURCES_LITE.iter(),
        source_dir,
        &kj_source_dir,
        true,
    )?;

    // kj-async files break capnproto's own import conventions and are impossible to compile
    // seperately without significant header changes, so we compile it into the library as a feature.
    if cfg!(feature = "async") {
        stage_files(
            &mut build,
            KJ_ASYNC_SOURCES.iter(),
            source_dir,
            &kj_source_dir,
            true,
        )?;
    }

    if CAPNP_HEAVY {
        stage_files(
            &mut build,
            KJ_SOURCES_HEAVY.iter(),
            source_dir,
            &kj_source_dir,
            true,
        )?;
    }

    kj_configure(
        &mut build,
        CAPNP_HEAVY,
        cfg!(feature = "track_lock_blocking"),
        cfg!(feature = "save_acquired_lock_info"),
    );
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
