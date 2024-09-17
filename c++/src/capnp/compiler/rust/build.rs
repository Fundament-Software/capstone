use eyre::eyre;
use kj_build::kj_configure;
use kj_build::stage_files;
use std::{env, fs, path::Path};

static CAPNPC_SOURCES: &[&str] = &[
    "compiler/type-id.c++",
    "compiler/error-reporter.c++",
    "compiler/lexer.capnp.c++",
    "compiler/lexer.c++",
    "compiler/grammar.capnp.c++",
    "compiler/parser.c++",
    "compiler/generics.c++",
    "compiler/node-translator.c++",
    "compiler/compiler.c++",
    //"compiler/capnp.c++",
    "compiler/module-loader.c++",
];
static CAPNPC_EXTRAS: &[&str] = &["compiler/lexer.capnp.h", "compiler/grammar.capnp.h"];
static CAPNPC_HEADERS: &[&str] = &[
    "compiler/type-id.h",
    "compiler/error-reporter.h",
    "compiler/lexer.capnp.h",
    "compiler/lexer.h",
    "compiler/grammar.capnp.h",
    "compiler/parser.h",
    "compiler/generics.h",
    "compiler/node-translator.h",
    "compiler/compiler.h",
    "compiler/module-loader.h",
    "compiler/resolver.h",
];
static CAPNPC_PRIVATE_HEADERS: &[&str] = &["../kj/miniposix.h"];

fn main() -> eyre::Result<()> {
    let out_dir = env::var_os("OUT_DIR").ok_or_else(|| eyre!("OUT_DIR not set"))?;
    let sources = Path::new(&out_dir).join("sources");
    let source_dir = Path::new("../..");
    let capnpc_source_dir = sources.join("capnp");

    //let _ = fs::remove_dir_all(&capnpc_source_dir);
    fs::create_dir_all(sources.join("kj"))?;
    fs::create_dir_all(capnpc_source_dir.join("compiler"))?;

    cxx_build::CFG.exported_header_dirs.push(&sources);
    cxx_build::CFG.include_prefix = "capnp";
    let mut build = cxx_build::bridge("lib.rs");

    stage_files(
        &mut build,
        CAPNPC_HEADERS
            .iter()
            .chain(CAPNPC_PRIVATE_HEADERS)
            .chain(CAPNPC_EXTRAS),
        source_dir,
        &capnpc_source_dir,
        false,
    )?;

    stage_files(
        &mut build,
        CAPNPC_SOURCES.iter(),
        source_dir,
        &capnpc_source_dir,
        true,
    )?;

    stage_files(
        &mut build,
        ["glue.h"].iter(),
        ".",
        &capnpc_source_dir,
        false,
    )?;

    stage_files(
        &mut build,
        ["jank.c++", "glue.c++"].iter(),
        ".",
        &capnpc_source_dir,
        true,
    )?;

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

    build.include(capnpc_source_dir);
    build.opt_level(3);
    build.warnings(false).std("c++20").compile("capnpc");

    Ok(())
}
