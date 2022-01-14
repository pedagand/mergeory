use clap::{App, Arg};
use std::cmp::min;
use std::ffi::OsStr;
use std::fs::read;
use std::path::Path;
use std::process::exit;
use syndiff::{
    add_extra_blocks, apply_patch, canonicalize_metavars, compute_diff, count_conflicts,
    merge_diffs, parse_source, remove_metavars, AnsiColoredTreeFormatter, Color,
    PlainTreeFormatter, SynNode, TextColoredTreeFormatter,
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
        .arg(Arg::with_name("colored").short("c").long("colored").help("Display difference node colors"))
        .arg(Arg::with_name("text-colored").short("C").long("text-colored").help("Display difference node colors as plain text without ANSI color codes"))
        .arg(Arg::with_name("merge-files").short("m").long("merge-files").requires("second-modified-file").help("If there are no conflicts, print the resulting merged file instead of the merged difference"))
        .arg(Arg::with_name("quiet").short("q").long("quiet").requires("second-modified-file").help("Do not print anything, just compute the number of conflicts"))
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

    let extra_blocks = cmd_args.is_present("extra-blocks");

    let origin_src = read_file(&origin_filename);
    let origin_tree = parse_tree(&origin_src, &origin_filename, &mut parser, extra_blocks);

    let first_modified_filename = cmd_args.value_of_os("first-modified-file").unwrap();
    let first_modified_src = read_file(&first_modified_filename);
    let first_modified_tree = parse_tree(
        &first_modified_src,
        &first_modified_filename,
        &mut parser,
        extra_blocks,
    );

    match cmd_args.value_of_os("second-modified-file") {
        None => {
            let no_elisions =
                cmd_args.is_present("no-elisions") || cmd_args.is_present("standalone");
            compute_diff(
                &origin_tree,
                &first_modified_tree,
                no_elisions,
                Color::try_from(0).unwrap(),
            )
            .write_with(&mut PlainTreeFormatter::new(std::io::stdout().lock()))
            .unwrap_or_else(|err| {
                eprintln!("Unable to write output: {}", err);
                exit(-1)
            })
        }
        Some(second_modified_filename) => {
            let second_modified_src = read_file(&second_modified_filename);
            let second_modified_tree = parse_tree(
                &second_modified_src,
                &second_modified_filename,
                &mut parser,
                extra_blocks,
            );

            let first_diff = compute_diff(
                &origin_tree,
                &first_modified_tree,
                cmd_args.is_present("no-elisions"),
                Color::try_from(0).unwrap(),
            );
            let second_diff = compute_diff(
                &origin_tree,
                &second_modified_tree,
                cmd_args.is_present("no-elisions"),
                Color::try_from(1).unwrap(),
            );

            let mut merged_diff = merge_diffs(first_diff, second_diff).unwrap();
            canonicalize_metavars(&mut merged_diff);
            let nb_conflicts = count_conflicts(&merged_diff);

            if !cmd_args.is_present("quiet") {
                if nb_conflicts == 0 && cmd_args.is_present("merge-files") {
                    let merged_tree = apply_patch(merged_diff, &origin_tree).unwrap();
                    merged_tree
                        .write_with(&mut PlainTreeFormatter::new(std::io::stdout().lock()))
                        .unwrap_or_else(|err| {
                            eprintln!("Unable to write output: {}", err);
                            exit(-1)
                        });
                } else {
                    let out_tree = if cmd_args.is_present("standalone") {
                        remove_metavars(merged_diff, &origin_tree).unwrap()
                    } else {
                        merged_diff
                    };
                    if cmd_args.is_present("colored") || cmd_args.is_present("text-colored") {
                        if cmd_args.is_present("text-colored") {
                            out_tree.write_with(&mut TextColoredTreeFormatter::new(
                                std::io::stdout().lock(),
                            ))
                        } else {
                            out_tree.write_with(&mut AnsiColoredTreeFormatter::new(
                                std::io::stdout().lock(),
                            ))
                        }
                    } else {
                        out_tree.write_with(&mut PlainTreeFormatter::new(std::io::stdout().lock()))
                    }
                    .unwrap_or_else(|err| {
                        eprintln!("Unable to write output: {}", err);
                        exit(-1)
                    });
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
    extra_blocks: bool,
) -> SynNode<'t> {
    let origin_tree = parse_source(source, parser).unwrap_or_else(|| {
        eprintln!("Unable to parse {}", filename.to_string_lossy());
        exit(-2)
    });
    if extra_blocks {
        add_extra_blocks(&origin_tree)
    } else {
        origin_tree
    }
}
