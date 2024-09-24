use eyre::eyre;
use kj_build::kj_configure;
use std::{env, path::Path};

const CAPNP_HEAVY: bool = cfg!(feature = "heavy");

static CAPNP_SOURCES_LITE: &[&str] = &[
    "any.c++",
    "arena.c++",
    "blob.c++",
    "c++.capnp.c++",
    "layout.c++",
    "list.c++",
    "message.c++",
    "schema.capnp.c++",
    "serialize.c++",
    "serialize-packed.c++",
    "stream.capnp.c++",
];
static CAPNP_SOURCES_HEAVY: &[&str] = &[
    "dynamic.c++",
    "schema.c++",
    "schema-loader.c++",
    "stringify.c++",
];
static CAPNP_EXTRAS: &[&str] = &[
    "c++.capnp.h",
    "schema.capnp.h",
    "stream.capnp.h",
    "schema-parser.c++",
    "serialize-text.c++",
];
static CAPNP_HEADERS: &[&str] = &[
    "any.h",
    "blob.h",
    "c++.capnp.h",
    "capability.h",
    "common.h",
    "dynamic.h",
    "endian.h",
    "generated-header-support.h",
    "layout.h",
    "list.h",
    "membrane.h",
    "message.h",
    "orphan.h",
    "persistent.capnp.h",
    "pointer-helpers.h",
    "pretty-print.h",
    "raw-schema.h",
    "schema.capnp.h",
    "schema.h",
    "schema-lite.h",
    "schema-loader.h",
    "schema-parser.h",
    "serialize.h",
    "serialize-async.h",
    "serialize-packed.h",
    "serialize-text.h",
    "stream.capnp.h",
    "test-util.h",
];
static CAPNP_PRIVATE_HEADERS: &[&str] = &["arena.h"];
static CAPNP_COMPAT_HEADERS: &[&str] = &["compat/std-iterator.h"];

fn main() -> eyre::Result<()> {
    let out_dir = env::var_os("OUT_DIR").ok_or_else(|| eyre!("OUT_DIR not set"))?;
    let source_dir = Path::new(&out_dir)
        .join("cxxbridge")
        .join("crate")
        .join("capnp");

    cxx_build::CFG.include_prefix = "capnp";
    let mut build = cxx_build::bridge("lib.rs");

    CAPNP_HEADERS
        .iter()
        .chain(CAPNP_COMPAT_HEADERS)
        .chain(CAPNP_PRIVATE_HEADERS)
        .chain(CAPNP_EXTRAS)
        .for_each(|s| println!("cargo:rerun-if-changed={}", s));

    CAPNP_SOURCES_LITE
        .iter()
        .chain(if CAPNP_HEAVY {
            CAPNP_SOURCES_HEAVY
        } else {
            &[]
        })
        .map(|s| (s, source_dir.join(s)))
        .for_each(|(s, p)| {
            println!("cargo:rerun-if-changed={}", s);
            // This copy is only here in case the symlink fails on windows
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            let _ = std::fs::copy(Path::new(s), &p);
            build.file(p);
        });

    // Unfuck MSVC
    build.flag_if_supported("/Zc:__cplusplus");
    build.flag_if_supported("/EHsc");
    build.flag_if_supported("/TP");

    kj_configure(
        &mut build,
        CAPNP_HEAVY,
        cfg!(feature = "track_lock_blocking"),
        cfg!(feature = "save_acquired_lock_info"),
    );
    println!("cargo:rustc-link-lib=kj");
    #[cfg(not(target_os = "windows"))]
    println!("cargo:rustc-link-lib=pthread");
    if cfg!(feature = "libdl") {
        println!("cargo:rustc-link-lib=dl");
    }

    build.opt_level(3);
    build.warnings(false).std("c++20").compile("capnp");

    Ok(())
}
