mod diff;
mod generic_tree;
mod merge;
mod syn_tree;
mod tree_formatter;

pub use crate::diff::{compute_diff, Metavariable, SpineNode as DiffSpineNode};
pub use crate::merge::{
    apply_patch, canonicalize_metavars, count_conflicts, merge_diffs, merge_multi_diffs,
    remove_metavars, SpineNode as MergedSpineNode,
};
pub use crate::syn_tree::{add_extra_blocks, parse_source, SynNode};
pub use crate::tree_formatter::{ColoredTreeFormatter, PlainTreeFormatter, TreeFormatter};
