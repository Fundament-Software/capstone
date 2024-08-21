#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!(<capnp/compiler/compiler.h>);
        include!(<capnp/glue.h>);

        fn command(
            files: &[String],
            imports: &[String],
            prefixes: &[String],
            standard_import: bool,
        ) -> Result<Vec<u8>>;
    }
}

pub fn call(
    files: impl Iterator<Item = impl AsRef<str>>,
    imports: impl Iterator<Item = impl AsRef<str>>,
    prefixes: impl Iterator<Item = impl AsRef<str>>,
    standard_import: bool,
) -> Result<Vec<u8>, cxx::Exception> {
    let file_list: Vec<String> = files.map(|e| e.as_ref().to_string()).collect();
    let import_list: Vec<String> = imports.map(|e| e.as_ref().to_string()).collect();
    let prefix_list: Vec<String> = prefixes.map(|e| e.as_ref().to_string()).collect();

    ffi::command(
        file_list.as_slice(),
        import_list.as_slice(),
        prefix_list.as_slice(),
        standard_import,
    )
}
