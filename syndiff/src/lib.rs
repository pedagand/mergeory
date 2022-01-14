mod colors;
mod diff;
mod generic_tree;
mod merge;
mod syn_tree;
mod tree_formatter;

pub use crate::colors::{Color, ColorSet, Colored};
pub use crate::diff::{compute_diff, DiffSpineNode, Metavariable};
pub use crate::merge::{
    apply_patch, canonicalize_metavars, count_conflicts, merge_diffs, remove_metavars,
    MergedSpineNode,
};
pub use crate::syn_tree::{add_extra_blocks, parse_source, SynNode};
pub use crate::tree_formatter::{
    AnsiColoredTreeFormatter, PlainTreeFormatter, TextColoredTreeFormatter, TreeFormatter,
};
