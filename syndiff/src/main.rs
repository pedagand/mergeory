use std::env;
use std::fs;
use std::process;

use quote::quote;
use std::io::Write;
use std::process::{Command, Stdio};
use syndiff::multi_diff_tree::remove_metavars;
use syndiff::source_repr::source_repr;
use syndiff::{compute_diff, merge_diffs};

#[derive(PartialEq, Eq, Copy, Clone)]
enum OutputMode {
    Diff,
    StandaloneDiff,
}

fn main() {
    let mut origin_filename = None;
    let mut modified_filenames = Vec::new();
    let mut output_mode = OutputMode::Diff;
    for arg in env::args().skip(1) {
        match arg.as_ref() {
            "-d" | "--diff" => output_mode = OutputMode::Diff,
            "-s" | "--standalone" => output_mode = OutputMode::StandaloneDiff,
            _ => {
                if origin_filename.is_none() {
                    origin_filename = Some(arg);
                } else {
                    modified_filenames.push(arg);
                }
            }
        }
    }

    let origin_filename = origin_filename.unwrap_or_else(|| {
        eprintln!("Usage: syndiff <original_file> <modified_files>*");
        process::exit(1);
    });
    let origin_src = parse_src(&origin_filename);

    let mut diff_trees = Vec::new();
    for modified_filename in modified_filenames {
        let modified_src = parse_src(&modified_filename);
        diff_trees.push(compute_diff(origin_src.clone(), modified_src))
    }

    if diff_trees.is_empty() {
        eprintln!("Usage: syndiff <original_file> <modified_files>*");
        process::exit(1);
    }

    let result_tree: syn::File = if diff_trees.len() == 1 && output_mode == OutputMode::Diff {
        source_repr(diff_trees.pop().unwrap())
    } else {
        let merged_diffs = merge_diffs(diff_trees).unwrap();
        let out_tree = match output_mode {
            OutputMode::Diff => merged_diffs,
            OutputMode::StandaloneDiff => remove_metavars(merged_diffs, origin_src).unwrap(),
        };
        source_repr(out_tree)
    };

    // Pretty print the result
    let mut rustfmt = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to start rustfmt");
    let rustfmt_in = rustfmt
        .stdin
        .as_mut()
        .expect("Failed to open rustfmt stdin");
    write!(rustfmt_in, "{}", quote!(#result_tree)).unwrap();
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
        process::exit(1)
    })
}
