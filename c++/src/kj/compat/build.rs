use eyre::eyre;
use kj_build::kj_configure;
use kj_build::stage_files;
use std::{env, fs, path::Path};

static KJ_GZIP_SOURCES: &[&str] = &["compat/gzip.c++"];
static KJ_GZIP_HEADERS: &[&str] = &["compat/gzip.h"];

fn main() -> eyre::Result<()> {
    let out_dir = env::var_os("OUT_DIR").ok_or_else(|| eyre!("OUT_DIR not set"))?;
    let sources = Path::new(&out_dir).join("sources");
    let source_dir = Path::new("..");
    let kj_source_dir = sources.join("kj");

    fs::create_dir_all(kj_source_dir.join("compat"))?;

    //cxx_build::CFG.exported_header_dirs.push(&sources);
    cxx_build::CFG.include_prefix = "kj";
    let mut build = cxx_build::bridge("lib.rs");

    stage_files(
        &mut build,
        KJ_GZIP_HEADERS.into_iter(),
        source_dir,
        &kj_source_dir,
        false,
    )?;

    stage_files(
        &mut build,
        KJ_GZIP_SOURCES.into_iter(),
        source_dir,
        &kj_source_dir,
        true,
    )?;

    if cfg!(feature = "zlib") {
        build.define("KJ_HAS_ZLIB", None);
    }
    // Unfuck MSVC
    build.flag_if_supported("/Zc:__cplusplus");
    build.flag_if_supported("/EHsc");
    build.flag_if_supported("/TP");

    kj_configure(
        &mut build,
        cfg!(feature = "heavy"),
        cfg!(feature = "track_lock_blocking"),
        cfg!(feature = "save_acquired_lock_info"),
    );
    build.include(sources);
    build.warnings(false).std("c++20").compile("kj-gzip");

    Ok(())
}
