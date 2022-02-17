use clap::{App, Arg};
use std::cmp::min;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::read;
use std::path::Path;
use std::process::exit;
use syndiff::{
    add_extra_blocks, apply_patch, canonicalize_metavars, compute_diff, count_conflicts,
    merge_diffs, parse_source, remove_metavars, AnsiColoredTreeFormatter, MergeOptions,
    PlainTreeFormatter, SynNode, TextColoredTreeFormatter, TreeFormattable, MINIMAL_ALIGNMENT,
    PATIENCE_ALIGNMENT,
};
use tree_sitter::Parser;
use tree_sitter_config::Config;
use tree_sitter_loader::Loader;

fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("syndiff {}", panic_info);
        exit(-3)
    }));

    let cmd_args = App::new("Syndiff")
        .version("0.2")
        .author("Guillaume Bertholon <guillaume.bertholon@ens.fr>")
        .about("Compare files syntactically and merge file differences")
        .long_about("Compare files syntactically and merge file differences\n\n\
            If only two files are given, compute a difference between them \
            following their syntax tree and including code moves.\n\
            If three files are given, compute differences between the two modified files \
            and the original and then merge these differences.\n\
            Exit with the number of conflicts found during the merge (capped to 127).\n\n\
            Syntax trees are parsed from the provided source files by a tree-sitter grammar.")
        .arg(
            Arg::with_name("original-file")
                .required(true)
                .help("Path to the original file to diff"),
        )
        .arg(
            Arg::with_name("first-modified-file")
                .required(true)
                .help("Path to the first modified file")
        )
        .arg(Arg::with_name("second-modified-file").required(false).help("Path of the second modified file. If provided, perform a three-way merge."))
        .arg(Arg::with_name("standalone").short("s").long("standalone").help("Remove all elisions and unchanged nodes in the final output"))
        .arg(Arg::with_name("no-elisions").long("no-elisions").help("Do not try to elide moved code when computing diff"))
        .arg(Arg::with_name("elision-whitelist").long("elision-whitelist").takes_value(true).conflicts_with("no-elisions").help("Only try to perform elisions on nodes with a tree-sitter kind listed in the whitelist file"))
        .arg(Arg::with_name("colored").short("c").long("colored").help("Display difference node colors"))
        .arg(Arg::with_name("text-colored").short("C").long("text-colored").help("Display difference node colors as plain text without ANSI color codes"))
        .arg(Arg::with_name("merge-files").short("m").long("merge-files").requires("second-modified-file").help("If there are no conflicts, print the resulting merged file instead of the merged difference"))
        .arg(Arg::with_name("allow-nested-deletions").short("d").long("allow-nested-deletions").requires("second-modified-file").help("Accept to merge a deletion nested into another deletion without conflict"))
        .arg(Arg::with_name("ordered-insertions").short("o").long("ordered-insertions").requires("second-modified-file").help("Do not create insert order conflicts by always placing insertions in the first modified file before those of the second modified file"))
        .arg(Arg::with_name("quiet").short("q").long("quiet").requires("second-modified-file").help("Do not print anything, just compute the number of conflicts"))
        .arg(Arg::with_name("scope").long("scope").takes_value(true).help("Select the tree-sitter language by scope instead of file extension"))
        .arg(Arg::with_name("extra-blocks").short("b").long("extra-blocks").help("Add extra structure with additional blocks separated by empty lines"))
        .arg(Arg::with_name("ignore-whitespace").short("w").long("ignore-whitespace").help("Ignore differences in whitespace, take the spacing of the first modified file when a choice has to be made"))
        .arg(Arg::with_name("patience").long("patience").help("Use the patience diff algorithm for subtree sequences"))
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

    let ignore_whitespace = cmd_args.is_present("ignore-whitespace");
    let extra_blocks = cmd_args.is_present("extra-blocks");
    let elision_whitelist = if cmd_args.is_present("no-elisions") {
        Some(HashSet::new())
    } else if let Some(whitelist_filename) = cmd_args.value_of_os("elision-whitelist") {
        let whitelist_file = read_file(whitelist_filename);
        let mut whitelist = HashSet::new();
        for kind_str in whitelist_file.split(|c| char::from(*c).is_ascii_whitespace()) {
            let kind_str = String::from_utf8_lossy(kind_str);
            let kind_id = language.id_for_node_kind(&kind_str, true);
            if kind_id == 0 {
                eprintln!("Unknown node kind `{}` for parser", kind_str);
                exit(-2);
            }
            whitelist.insert(kind_id);
        }
        Some(whitelist)
    } else {
        None
    };
    let align_subtree_algorithm = if cmd_args.is_present("patience") {
        PATIENCE_ALIGNMENT
    } else {
        MINIMAL_ALIGNMENT
    };
    let color_mode = if cmd_args.is_present("text-colored") {
        ColorMode::TextColored
    } else if cmd_args.is_present("colored") {
        ColorMode::AnsiColored
    } else {
        ColorMode::NoColors
    };

    let origin_src = read_file(origin_filename);
    let origin_tree = parse_tree(
        &origin_src,
        origin_filename,
        &mut parser,
        ignore_whitespace,
        extra_blocks,
    );

    let first_modified_filename = cmd_args.value_of_os("first-modified-file").unwrap();
    let first_modified_src = read_file(first_modified_filename);
    let first_modified_tree = parse_tree(
        &first_modified_src,
        first_modified_filename,
        &mut parser,
        ignore_whitespace,
        extra_blocks,
    );

    match cmd_args.value_of_os("second-modified-file") {
        None => {
            let diff_tree = compute_diff(
                &origin_tree,
                &first_modified_tree,
                &elision_whitelist,
                align_subtree_algorithm,
            );
            if cmd_args.is_present("standalone") {
                let standalone_tree = remove_metavars(
                    merge_diffs(&diff_tree, &diff_tree, MergeOptions::default()).unwrap(),
                    &origin_tree,
                )
                .unwrap();
                print_tree(&standalone_tree, color_mode);
            } else {
                print_tree(&diff_tree, color_mode);
            }
        }
        Some(second_modified_filename) => {
            let second_modified_src = read_file(second_modified_filename);
            let second_modified_tree = parse_tree(
                &second_modified_src,
                second_modified_filename,
                &mut parser,
                ignore_whitespace,
                extra_blocks,
            );

            let first_diff = compute_diff(
                &origin_tree,
                &first_modified_tree,
                &elision_whitelist,
                align_subtree_algorithm,
            );
            let second_diff = compute_diff(
                &origin_tree,
                &second_modified_tree,
                &elision_whitelist,
                align_subtree_algorithm,
            );

            let mut merged_diff = merge_diffs(
                &first_diff,
                &second_diff,
                MergeOptions {
                    allow_nested_deletions: cmd_args.is_present("allow-nested-deletions"),
                    ordered_insertions: cmd_args.is_present("ordered-insertions"),
                },
            )
            .unwrap();
            canonicalize_metavars(&mut merged_diff);
            let nb_conflicts = count_conflicts(&merged_diff);

            if !cmd_args.is_present("quiet") {
                if nb_conflicts == 0 && cmd_args.is_present("merge-files") {
                    let merged_tree = apply_patch(merged_diff, &origin_tree).unwrap();
                    print_tree(&merged_tree, color_mode);
                } else {
                    let out_tree = if cmd_args.is_present("standalone") {
                        remove_metavars(merged_diff, &origin_tree).unwrap()
                    } else {
                        merged_diff
                    };

                    print_tree(&out_tree, color_mode);
                }
            }

            exit(min(nb_conflicts, 127).try_into().unwrap())
        }
    }
}

fn read_file(filename: &OsStr) -> Vec<u8> {
    read(filename).unwrap_or_else(|err| {
        eprintln!("Unable to read {}: {}", filename.to_string_lossy(), err);
        exit(-1)
    })
}

fn parse_tree<'t>(
    source: &'t [u8],
    filename: &OsStr,
    parser: &mut Parser,
    ignore_whitespace: bool,
    extra_blocks: bool,
) -> SynNode<'t> {
    let origin_tree = parse_source(source, parser, ignore_whitespace).unwrap_or_else(|| {
        eprintln!("Unable to parse {}", filename.to_string_lossy());
        exit(-2)
    });
    if extra_blocks {
        add_extra_blocks(&origin_tree)
    } else {
        origin_tree
    }
}

enum ColorMode {
    NoColors,
    TextColored,
    AnsiColored,
}

fn print_tree<T: TreeFormattable>(tree: &T, color_mode: ColorMode) {
    match color_mode {
        ColorMode::NoColors => {
            tree.write_with(&mut PlainTreeFormatter::new(std::io::stdout().lock()))
        }
        ColorMode::TextColored => {
            tree.write_with(&mut TextColoredTreeFormatter::new(std::io::stdout().lock()))
        }
        ColorMode::AnsiColored => {
            tree.write_with(&mut AnsiColoredTreeFormatter::new(std::io::stdout().lock()))
        }
    }
    .unwrap_or_else(|err| {
        eprintln!("Unable to write output: {}", err);
        exit(-1)
    });
}
