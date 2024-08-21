#[cxx::bridge(namespace = "kj")]
mod gzip {
    unsafe extern "C++" {
        include!("kj/compat/gzip.h");

        type GzipInputStream;
        type GzipOutputStream;
    }
}
