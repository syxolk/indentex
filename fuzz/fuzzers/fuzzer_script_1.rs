#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate indentex;

fuzz_target!(|data: &[u8]| {
    use indentex::transpile::{transpile_file, TranspileOptions};
    use indentex::file_utils::write_to_file;

    let path = "/tmp/test.inden.tex";

    write_to_file(&path, data);
    transpile_file(path, &TranspileOptions {
        flatten_output: false,
        prepend_do_not_edit_notice: false,
    });
});
