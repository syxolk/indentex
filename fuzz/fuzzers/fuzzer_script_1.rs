#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate indentex;

fuzz_target!(|data: &[u8]| {
    /*use std::process::Command;

    use std::fs::File;
    use std::io::{BufWriter, Write};

    let path = "/tmp/test.inden.tex";
    let file = File::create(path.as_ref())?;
    let mut buf = BufWriter::new(file);
    buf.write_all(data.as_ref().as_bytes())?;

    Command::new("indentex")
            .args(&[path])
            .output()
            .expect("failed to execute process");*/

    use indentex::transpile::{transpile_file, TranspileOptions};
    use indentex::file_utils::write_to_file;

    let path = "/tmp/test.inden.tex";

    write_to_file(&path, data);
    transpile_file(path, &TranspileOptions {
        flatten_output: false,
        prepend_do_not_edit_notice: false,
    });
});
