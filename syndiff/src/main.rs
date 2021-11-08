use std::cmp::min;
use std::fs::read;
use std::path::Path;
use std::process::exit;
use syndiff::{
    apply_patch, compute_diff, count_conflicts, merge_diffs, parse_source, remove_metavars,
};
use tree_sitter::Parser;
use tree_sitter_config::Config;
use tree_sitter_loader::Loader;

fn main() {
    // TODO: Use clap for argument parsing
    let mut origin_filename = None;
    let mut modified_filenames = Vec::new();
    let mut standalone_mode = false;
    //let mut colored_mode = false;
    let mut quiet = false;
    let mut merge_files_mode = false;
    for arg in std::env::args().skip(1) {
        match arg.as_ref() {
            "-s" | "--standalone" => standalone_mode = true,
            //"-c" | "--colored" => colored_mode = true,
            "-m" | "--merge-files" => merge_files_mode = true,
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

    let config = Config::load().unwrap();
    let mut lang_loader = Loader::new().unwrap();
    lang_loader
        .find_all_languages(&config.get().unwrap())
        .unwrap_or_else(|err| {
            eprintln!("Error loading parser list: {}", err);
            exit(-2)
        });

    let origin_filename = origin_filename.unwrap_or_else(|| {
        eprintln!("Usage: syndiff <original_file> <modified_files>*");
        exit(-1);
    });

    let (language, _lang_config) = lang_loader
        .language_configuration_for_file_name(Path::new(&origin_filename))
        .unwrap_or_else(|err| {
            eprintln!("Error loading parser: {}", err);
            exit(-2)
        })
        .unwrap_or_else(|| {
            eprintln!("No parser found for file {}", origin_filename);
            exit(-2)
        });

    let mut parser = Parser::new();
    parser.set_language(language).unwrap_or_else(|err| {
        eprintln!("Failed initializing parser: {}", err);
        exit(-2);
    });

    let origin_src = read(&origin_filename).unwrap_or_else(|err| {
        eprintln!("Unable to read {}: {}", origin_filename, err);
        exit(-1)
    });
    let origin_tree = parse_source(&origin_src, &mut parser).unwrap_or_else(|| {
        eprintln!("Unable to parse {}", origin_filename);
        exit(-2)
    });

    let modified_src: Vec<_> = modified_filenames
        .iter()
        .map(|filename| {
            read(filename).unwrap_or_else(|err| {
                eprintln!("Unable to read {}: {}", filename, err);
                exit(-1)
            })
        })
        .collect();

    let diff_trees: Vec<_> = modified_src
        .iter()
        .zip(&modified_filenames)
        .map(|(src, filename)| {
            let tree = parse_source(src, &mut parser).unwrap_or_else(|| {
                eprintln!("Unable to parse {}", filename);
                exit(-2)
            });
            compute_diff(&origin_tree, &tree)
        })
        .collect();

    if diff_trees.is_empty() {
        eprintln!("Usage: syndiff <original_file> <modified_files>*");
        exit(-1);
    }

    if diff_trees.len() == 1 && !standalone_mode && !merge_files_mode {
        diff_trees
            .into_iter()
            .next()
            .unwrap()
            .write_to(&mut std::io::stdout().lock())
            .unwrap_or_else(|err| {
                eprintln!("Unable to write output: {}", err);
                exit(-1)
            })
    } else {
        let merged_diffs = merge_diffs(&diff_trees).unwrap();
        let nb_conflicts = count_conflicts(&merged_diffs);

        if !quiet {
            if nb_conflicts == 0 && merge_files_mode {
                let merged_tree = apply_patch(merged_diffs, &origin_tree).unwrap();
                merged_tree
                    .write_to(&mut std::io::stdout().lock())
                    .unwrap_or_else(|err| {
                        eprintln!("Unable to write output: {}", err);
                        exit(-1)
                    });
            } else {
                let out_tree = if standalone_mode {
                    remove_metavars(merged_diffs, &origin_tree).unwrap()
                } else {
                    merged_diffs
                };
                out_tree
                    .write_to(&mut std::io::stdout().lock())
                    .unwrap_or_else(|err| {
                        eprintln!("Unable to write output: {}", err);
                        exit(-1)
                    });
            }
        }

        exit(min(nb_conflicts, 127).try_into().unwrap())
    }
}
