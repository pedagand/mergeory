use std::env;
use std::fs;
use std::process;

mod ast;
mod convert;
mod ellided_tree;
mod hash_tree;
mod merge;
mod patch_tree;
mod scoped_tree;
mod visit;

use convert::Convert;
use ellided_tree::{Ellider, WantedEllisionFinder};
use hash_tree::{tables_intersection, HashTables};
use merge::Merge;
use patch_tree::SpineZipper;
use scoped_tree::ComputeScopes;
use visit::Visit;

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

    let file_change = compute_change(&origin_src, &modified_src);

    // Merge the deletion and the insertion tree on their common part to create
    // a spine of unchanged structure.
    let diff_ast = zip_spine(file_change);

    // TODO: Compute a good diff representation
    println!("{:?}", diff_ast);
}

pub struct FileChange {
    deletion_ast: ast::ellided::File,
    insertion_ast: ast::ellided::File,
}

pub fn compute_change(origin_src: &SourceCode, modified_src: &SourceCode) -> FileChange {
    // Parse both input files and hash their AST
    let (origin_hash_ast, origin_hash_tables) = parse_and_hash_src(origin_src);
    let (modified_hash_ast, modified_hash_tables) = parse_and_hash_src(modified_src);

    // Find the common subtrees between original and modified versions
    let unified_hash_tables = tables_intersection(origin_hash_tables, modified_hash_tables);

    // Find which of the common subtrees will actually be ellided in both trees.
    // This avoids ellided part appearing only inside one of the subtrees.
    let mut origin_ellision_finder = WantedEllisionFinder::new(&unified_hash_tables);
    origin_ellision_finder.visit(&origin_hash_ast);
    let mut modified_ellision_finder = WantedEllisionFinder::new(&unified_hash_tables);
    modified_ellision_finder.visit(&modified_hash_ast);
    let ellision_tables = tables_intersection(
        origin_ellision_finder.wanted_ellisions,
        modified_ellision_finder.wanted_ellisions,
    );

    // Remove any reference to subtrees inside unified_hash_tables such that either:
    // * The subtree must be ellided
    // * The Rc counter of the subtree is 1
    drop(unified_hash_tables);

    // Compute the difference as a deletion and an insertion tree by elliding
    // parts reused from original to modified
    let deletion_ast = Ellider::new(&ellision_tables).convert(origin_hash_ast);
    let insertion_ast = Ellider::new(&ellision_tables).convert(modified_hash_ast);

    FileChange {
        deletion_ast,
        insertion_ast,
    }
}

pub fn zip_spine(changes: FileChange) -> ast::patch::File {
    let scoped_del = ComputeScopes::default().convert(changes.deletion_ast);
    let scoped_ins = ComputeScopes::default().convert(changes.insertion_ast);
    SpineZipper.merge(scoped_del, scoped_ins)
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

    let mut hash_tables = HashTables::default();
    let hash_ast: ast::hash::File = hash_tables.convert(ast);
    (hash_ast, hash_tables)
}
