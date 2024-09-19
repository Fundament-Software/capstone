use eyre::eyre;
use eyre::Result;
use std::path::Path;

pub fn stage_files(
    build: &mut cc::Build,
    files: impl Iterator<Item = impl AsRef<str>>,
    from: impl AsRef<Path>,
    to: impl AsRef<Path>,
    compile: bool,
) -> Result<()> {
    for file in files {
        let path = from.as_ref().join(file.as_ref());
        println!(
            "cargo:rerun-if-changed={}",
            path.to_str()
                .ok_or_else(|| eyre!("non–UTF-8 path: {:?}", path))?
        );
        let target = to.as_ref().join(file.as_ref());
        if !(std::fs::exists(&target)?) {
            std::fs::copy(&path, &target).map_err(|e| {
                eyre!(
                    "Failed to copy {} to {}: {}",
                    path.display(),
                    target.display(),
                    e.to_string()
                )
            })?;
        }
        if compile {
            build.file(target);
        }
    }

    Ok(())
}

fn bool_int_str(p: bool) -> &'static str {
    if p {
        "1"
    } else {
        "0"
    }
}

pub fn kj_configure(build: &mut cc::Build, heavy: bool, track: bool, save: bool) -> &mut cc::Build {
    if !heavy {
        build.define("CAPNP_LITE", None);
    }
    build
        .define("KJ_SAVE_ACQUIRED_LOCK_INFO", bool_int_str(save))
        .define("KJ_TRACK_LOCK_BLOCKING", bool_int_str(track))
}
