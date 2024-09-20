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
