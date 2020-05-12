use std::env;
use std::fs;
use std::process;

use quote::quote;
use std::io::Write;
use std::process::{Command, Stdio};
use syndiff::compute_diff;
use syndiff::source_repr::source_repr;

fn main() {
    let mut args = env::args().skip(1);
    let (origin_filename, modified_filename) = match (args.next(), args.next(), args.next()) {
        (Some(filename1), Some(filename2), None) => (filename1, filename2),
        _ => {
            eprintln!("Usage: syndiff <original_file> <new_file>");
            process::exit(1);
        }
    };

    let origin_src = parse_src(&origin_filename);
    let modified_src = parse_src(&modified_filename);

    let diff_tree = compute_diff(origin_src, modified_src);

    // Pretty print the result
    let source_diff_tree: syn::File = source_repr(diff_tree);
    let mut rustfmt = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to start rustfmt");
    let rustfmt_in = rustfmt
        .stdin
        .as_mut()
        .expect("Failed to open rustfmt stdin");
    write!(rustfmt_in, "{}", quote!(#source_diff_tree)).unwrap()
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
        process::exit(1)
    })
}
