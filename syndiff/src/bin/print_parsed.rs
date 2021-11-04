use quote::quote;
use std::env;
use std::fs;
use std::io::Write;
use std::process;
use std::process::{Command, Stdio};

fn main() {
    if env::args().len() != 2 {
        eprintln!("Usage: print_parsed <filename>");
        process::exit(1)
    }

    let filename = env::args().skip(1).next().unwrap();
    let src = parse_src(&filename);

    // Pretty print the result
    let mut rustfmt = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to start rustfmt");
    let rustfmt_in = rustfmt
        .stdin
        .as_mut()
        .expect("Failed to open rustfmt stdin");
    write!(rustfmt_in, "{}", quote!(#src)).unwrap();
    rustfmt.wait().unwrap();
}

fn parse_src(src_filename: &str) -> syn::File {
    let code = fs::read_to_string(src_filename).unwrap_or_else(|err| {
        eprintln!("Unable to read {}: {}", src_filename, err);
        process::exit(1)
    });

    syn::parse_file(&code).unwrap_or_else(|err| {
        let err_start = err.span().start();
        let err_end = err.span().end();
        if err_start.line == err_end.line {
            eprintln!(
                "File \"{}\", line {}, columns {}-{}:\n{}",
                src_filename, err_start.line, err_start.column, err_end.column, err
            )
        } else {
            eprintln!(
                "File \"{}\", lines {}-{}:\n{}",
                src_filename, err_start.line, err_end.line, err
            )
        }
        process::exit(2)
    })
}
