#[cxx::bridge(namespace = "capnp")]
mod common {
    unsafe extern "C++" {
        include!(<capnp/message.h>);
    }
}
