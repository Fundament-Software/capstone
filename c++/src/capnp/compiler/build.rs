use eyre::eyre;
use eyre::OptionExt;
use kj_build::kj_configure;
use std::{env, path::Path};

static CAPNPC_SOURCES: &[&str] = &[
    "type-id.c++",
    "error-reporter.c++",
    "lexer.capnp.c++",
    "lexer.c++",
    "grammar.capnp.c++",
    "parser.c++",
    "generics.c++",
    "node-translator.c++",
    "compiler.c++",
    //"capnp.c++",
    "module-loader.c++",
];
static CAPNPC_EXTRAS: &[&str] = &["lexer.capnp.h", "grammar.capnp.h"];
static CAPNPC_HEADERS: &[&str] = &[
    "type-id.h",
    "error-reporter.h",
    "lexer.capnp.h",
    "lexer.h",
    "grammar.capnp.h",
    "parser.h",
    "generics.h",
    "node-translator.h",
    "compiler.h",
    "module-loader.h",
    "resolver.h",
];

fn main() -> eyre::Result<()> {
    let out_dir = env::var_os("OUT_DIR").ok_or_else(|| eyre!("OUT_DIR not set"))?;
    let source_dir = Path::new(&out_dir)
        .join("cxxbridge")
        .join("crate")
        .join("compiler");

    cxx_build::CFG.include_prefix = "compiler";
    let mut build = cxx_build::bridge("lib.rs");

    CAPNPC_HEADERS
        .iter()
        .chain(CAPNPC_EXTRAS)
        .chain(["glue.h"].iter())
        .map(|s| source_dir.join(s))
        .try_for_each(|p| {
            println!(
                "cargo:rerun-if-changed={}",
                p.to_str().ok_or_eyre("non–UTF-8 path")?
            );
            Ok::<(), eyre::Report>(())
        })?;

    CAPNPC_SOURCES
        .iter()
        .chain(["jank.c++", "glue.c++"].iter())
        .map(|s| source_dir.join(s))
        .try_for_each(|p| {
            println!(
                "cargo:rerun-if-changed={}",
                p.to_str().ok_or_eyre("non–UTF-8 path")?
            );
            build.file(p);
            Ok::<(), eyre::Report>(())
        })?;

    // Unfuck MSVC
    build.flag_if_supported("/Zc:__cplusplus");
    build.flag_if_supported("/EHsc");
    build.flag_if_supported("/TP");

    kj_configure(
        &mut build,
        true,
        cfg!(feature = "track_lock_blocking"),
        cfg!(feature = "save_acquired_lock_info"),
    );
    println!("cargo:rustc-link-lib=capnp");
    println!("cargo:rustc-link-lib=kj");
    #[cfg(not(target_os = "windows"))]
    println!("cargo:rustc-link-lib=pthread");
    if cfg!(feature = "libdl") {
        println!("cargo:rustc-link-lib=dl");
    }

    build.opt_level(3);
    build.warnings(false).std("c++20").compile("capnpc");

    Ok(())
}
