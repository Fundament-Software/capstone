#[cxx::bridge(namespace = "kj")]
mod common {
    unsafe extern "C++" {
        include!(<kj/common.h>);
    }
}

#[cxx::bridge(namespace = "kj")]
mod main {
    unsafe extern "C++" {
        include!(<kj/main.h>);
    }
}
