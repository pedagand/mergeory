use std::cmp::min;
use std::convert::TryInto;
use std::env;
use std::fs;
use std::process;

use quote::quote;
use std::io::Write;
use std::process::{Command, Stdio};
use syndiff::multi_diff_tree::{apply_patch, count_conflicts, remove_metavars};
use syndiff::source_repr::{colored_source_repr, source_repr};
use syndiff::{compute_diff, merge_diffs};

fn main() {
    let mut origin_filename = None;
    let mut modified_filenames = Vec::new();
    let mut standalone_mode = false;
    let mut colored_mode = false;
    let mut quiet = false;
    let mut merged_files_mode = false;
    for arg in env::args().skip(1) {
        match arg.as_ref() {
            "-s" | "--standalone" => standalone_mode = true,
            "-c" | "--colored" => colored_mode = true,
            "-m" | "--merge-files" => merged_files_mode = true,
            "-q" | "--quiet" => quiet = true,
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
        process::exit(-1);
    });
    let origin_src = parse_src(&origin_filename);

    let mut diff_trees = Vec::new();
    for modified_filename in modified_filenames {
        let modified_src = parse_src(&modified_filename);
        diff_trees.push(compute_diff(origin_src.clone(), modified_src))
    }

    if diff_trees.is_empty() {
        eprintln!("Usage: syndiff <original_file> <modified_files>*");
        process::exit(-1);
    }

    let (result_tree, nb_conflicts): (syn::File, u64) =
        if diff_trees.len() == 1 && !standalone_mode && !merged_files_mode {
            (source_repr(diff_trees.pop().unwrap()), 0)
        } else {
            let merged_diffs = merge_diffs(diff_trees).unwrap();
            let nb_conflicts = count_conflicts(&merged_diffs);
            if nb_conflicts == 0 && merged_files_mode {
                (apply_patch(merged_diffs, origin_src).unwrap(), 0)
            } else {
                let out_tree = if standalone_mode {
                    remove_metavars(merged_diffs, origin_src).unwrap()
                } else {
                    merged_diffs
                };
                if colored_mode {
                    (colored_source_repr(out_tree), nb_conflicts)
                } else {
                    (source_repr(out_tree), nb_conflicts)
                }
            }
        };

    if !quiet {
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

    process::exit(min(nb_conflicts, 127).try_into().unwrap())
}

fn parse_src(src_filename: &str) -> syn::File {
    let code = fs::read_to_string(src_filename).unwrap_or_else(|err| {
        eprintln!("Unable to read {}: {}", src_filename, err);
        process::exit(-1)
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
        process::exit(-2)
    })
}
