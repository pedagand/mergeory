use std::env;
use std::fs;
use std::process;

mod ast;
mod convert;
mod hash_tree;

use convert::Convert;
use hash_tree::{tables_intersection, HashTables};

struct SourceCode {
    filename: String,
    code: String,
}

impl SourceCode {
    fn from_file(filename: String) -> std::io::Result<SourceCode> {
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
    diff_src(&origin_src, &modified_src)
}

fn diff_src(origin_src: &SourceCode, modified_src: &SourceCode) {
    let (origin_hash_ast, origin_hash_tables) = parse_and_hash_src(origin_src);
    let (modified_hash_ast, modified_hash_tables) = parse_and_hash_src(modified_src);

    let unified_hash_tables = tables_intersection(origin_hash_tables, modified_hash_tables);
}

fn parse_and_hash_src(src: &SourceCode) -> (ast::hash_tree::File, HashTables) {
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
    let hash_ast: ast::hash_tree::File = hash_tables.convert(ast);
    (hash_ast, hash_tables)
}
