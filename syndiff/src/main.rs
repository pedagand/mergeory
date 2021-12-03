use clap::{App, Arg};
use std::cmp::min;
use std::fs::read;
use std::path::Path;
use std::process::exit;
use syndiff::{
    add_extra_blocks, apply_patch, compute_diff, count_conflicts, merge_diffs, parse_source,
    remove_metavars,
};
use tree_sitter::Parser;
use tree_sitter_config::Config;
use tree_sitter_loader::Loader;

fn main() {
    let cmd_args = App::new("Syndiff")
        .version("0.2")
        .author("Guillaume Bertholon <guillaume.bertholon@ens.fr>")
        .about("Compare files syntactically and merge file differences")
        .long_about("Compare files syntactically and merge file differences\n\n\
            If only two files are given, compute a difference between them \
            following their syntax tree and including code moves.\n\
            If more than one modified file is given, compute all the differences between each \
            modified file and the original and then merge these differences.\n\
            Exit with the number of conflicts found during the merge (capped to 127).\n\n\
            Syntax trees are parsed from the provided source files by a tree-sitter grammar.")
        .arg(
            Arg::with_name("original-file")
                .required(true)
                .help("Path to the original file to diff"),
        )
        .arg(
            Arg::with_name("modified-file")
                .required(true)
                .multiple(true)
                .help("Path to the modified files to diff. If more than one is provided, then merge together the resulting differences")
        )
        .arg(Arg::with_name("standalone").short("s").long("standalone").help("Remove all elisions and unchanged nodes in the final output"))
        .arg(Arg::with_name("no-elisions").long("no-elisions").help("Do not try to elide moved code when computing diff"))
        .arg(Arg::with_name("colored").short("c").long("colored").help("Display difference node colors"))
        .arg(Arg::with_name("merge-files").short("m").long("merge-files").help("If there are no conflicts, print the resulting merged file instead of the merged difference"))
        .arg(Arg::with_name("quiet").short("q").long("quiet").help("Do not print anything, just compute the number of conflicts"))
        .arg(Arg::with_name("scope").long("scope").takes_value(true).help("Select the tree-sitter language by scope instead of file extension"))
        .arg(Arg::with_name("extra-blocks").short("b").long("extra-blocks").help("Add extra structure with additional blocks separated by empty lines"))
        .get_matches_safe()
        .unwrap_or_else(|err| {
            eprintln!("{}", err);
            exit(-1)
        });

    let config = Config::load().unwrap();
    let mut lang_loader = Loader::new().unwrap();
    lang_loader
        .find_all_languages(&config.get().unwrap())
        .unwrap_or_else(|err| {
            eprintln!("Error loading parser list: {}", err);
            exit(-2)
        });

    let origin_filename = cmd_args.value_of_os("original-file").unwrap();

    let language = lang_loader
        .select_language(
            Path::new(&origin_filename),
            Path::new(""),
            cmd_args.value_of("scope"),
        )
        .unwrap_or_else(|err| {
            eprintln!("Error loading parser: {}", err);
            exit(-2)
        });

    let mut parser = Parser::new();
    parser.set_language(language).unwrap_or_else(|err| {
        eprintln!("Failed initializing parser: {}", err);
        exit(-2);
    });

    let origin_src = read(&origin_filename).unwrap_or_else(|err| {
        eprintln!(
            "Unable to read {}: {}",
            origin_filename.to_string_lossy(),
            err
        );
        exit(-1)
    });
    let origin_tree = parse_source(&origin_src, &mut parser).unwrap_or_else(|| {
        eprintln!("Unable to parse {}", origin_filename.to_string_lossy());
        exit(-2)
    });
    let origin_tree = if cmd_args.is_present("extra-blocks") {
        add_extra_blocks(&origin_tree)
    } else {
        origin_tree
    };

    let modified_src: Vec<_> = cmd_args
        .values_of_os("modified-file")
        .unwrap()
        .map(|filename| {
            read(filename).unwrap_or_else(|err| {
                eprintln!("Unable to read {}: {}", filename.to_string_lossy(), err);
                exit(-1)
            })
        })
        .collect();

    let diff_trees: Vec<_> = modified_src
        .iter()
        .zip(cmd_args.values_of_lossy("modified-file").unwrap())
        .map(|(src, filename)| {
            let tree = parse_source(src, &mut parser).unwrap_or_else(|| {
                eprintln!("Unable to parse {}", filename);
                exit(-2)
            });
            let tree = if cmd_args.is_present("extra-blocks") {
                add_extra_blocks(&tree)
            } else {
                tree
            };
            compute_diff(&origin_tree, &tree, cmd_args.is_present("no-elisions"))
        })
        .collect();

    if diff_trees.len() == 1
        && !cmd_args.is_present("standalone")
        && !cmd_args.is_present("merge-files")
    {
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

        if !cmd_args.is_present("quiet") {
            if nb_conflicts == 0 && cmd_args.is_present("merge-files") {
                let merged_tree = apply_patch(merged_diffs, &origin_tree).unwrap();
                merged_tree
                    .write_to(&mut std::io::stdout().lock())
                    .unwrap_or_else(|err| {
                        eprintln!("Unable to write output: {}", err);
                        exit(-1)
                    });
            } else {
                let out_tree = if cmd_args.is_present("standalone") {
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
