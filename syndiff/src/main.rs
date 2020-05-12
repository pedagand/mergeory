use std::env;
use std::fs;
use std::process;

use quote::quote;
use std::io::Write;
use std::process::{Command, Stdio};
use syndiff::ast;
use syndiff::ellided_tree::{ellide_tree_with, find_wanted_ellisions};
use syndiff::hash_tree::{hash_tree, tables_intersection, HashTables};
use syndiff::patch_tree::zip_spine;
use syndiff::source_repr::source_repr;
use syndiff::weighted_tree::compute_weight;

pub struct SourceCode {
    filename: String,
    code: String,
}

impl SourceCode {
    pub fn from_file(filename: String) -> std::io::Result<SourceCode> {
        Ok(SourceCode {
            code: fs::read_to_string(&filename)?,
            filename,
        })
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let (origin_filename, modified_filename) = match (args.next(), args.next(), args.next()) {
        (Some(filename1), Some(filename2), None) => (filename1, filename2),
        _ => {
            eprintln!("Usage: syndiff <original_file> <new_file>");
            process::exit(1);
        }
    };

    let origin_src = SourceCode::from_file(origin_filename).expect("Unable to read origin file");
    let modified_src =
        SourceCode::from_file(modified_filename).expect("Unable to read modified file");

    let changes = compute_changes(&origin_src, &modified_src);

    // Merge the deletion and the insertion tree on their common part to create
    // a spine of unchanged structure.
    let weighted_del: ast::weighted::File = compute_weight(changes.deletion_ast);
    let weighted_ins: ast::weighted::File = compute_weight(changes.insertion_ast);
    let diff_ast = zip_spine(weighted_del, weighted_ins);

    // Pretty print the result
    let source_diff_ast: Option<syn::File> = source_repr(diff_ast);
    let source_diff_ast = source_diff_ast.expect("The two input files were not comparable");
    let mut rustfmt = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to start rustfmt");
    let rustfmt_in = rustfmt
        .stdin
        .as_mut()
        .expect("Failed to open rustfmt stdin");
    write!(rustfmt_in, "{}", quote!(#source_diff_ast)).unwrap()
}

pub struct FileChange {
    deletion_ast: ast::ellided::File,
    insertion_ast: ast::ellided::File,
}

pub fn compute_changes(origin_src: &SourceCode, modified_src: &SourceCode) -> FileChange {
    // Parse both input files and hash their AST
    let (origin_hash_ast, origin_hash_tables) = parse_and_hash_src(origin_src);
    let (modified_hash_ast, modified_hash_tables) = parse_and_hash_src(modified_src);

    // Find the common subtrees between original and modified versions
    let unified_hash_tables = tables_intersection(origin_hash_tables, modified_hash_tables);

    // Find which of the common subtrees will actually be ellided in both trees.
    // This avoids ellided part appearing only inside one of the subtrees.
    let origin_wanted_ellisions = find_wanted_ellisions(&origin_hash_ast, &unified_hash_tables);
    let modified_wanted_ellisions = find_wanted_ellisions(&modified_hash_ast, &unified_hash_tables);
    let ellision_tables = tables_intersection(origin_wanted_ellisions, modified_wanted_ellisions);

    // Remove any reference to subtrees inside unified_hash_tables such that either:
    // * The subtree must be ellided
    // * The Rc counter of the subtree is 1
    drop(unified_hash_tables);

    // Compute the difference as a deletion and an insertion tree by elliding
    // parts reused from original to modified
    let deletion_ast = ellide_tree_with(origin_hash_ast, &ellision_tables);
    let insertion_ast = ellide_tree_with(modified_hash_ast, &ellision_tables);

    FileChange {
        deletion_ast,
        insertion_ast,
    }
}

fn parse_and_hash_src(src: &SourceCode) -> (ast::hash::File, HashTables) {
    let ast = syn::parse_file(&src.code).unwrap_or_else(|err| {
        let err_start = err.span().start();
        let err_end = err.span().end();
        if err_start.line == err_end.line {
            eprintln!(
                "File \"{}\", line {}, columns {}-{}:\n{}",
                src.filename, err_start.line, err_start.column, err_end.column, err
            )
        } else {
            eprintln!(
                "File \"{}\", lines {}-{}:\n{}",
                src.filename, err_start.line, err_end.line, err
            )
        }
        process::exit(1)
    });

    hash_tree(ast)
}
