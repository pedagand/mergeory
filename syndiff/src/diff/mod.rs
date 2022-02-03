mod alignment;
mod elision;
mod tree;
mod weight;

pub use tree::Metavariable;
pub use tree::{ChangeNode, DiffSpineNode, DiffSpineSeqNode};

use crate::generic_tree::NodeKind;
use crate::syn_tree::SynNode;
use alignment::align_trees;
use elision::find_metavariable_elisions;
use std::collections::HashSet;
use weight::weight_tree;

pub fn compute_diff<'t>(
    origin_tree: &SynNode<'t>,
    modified_tree: &SynNode<'t>,
    kind_whitelist: &Option<HashSet<NodeKind>>,
) -> DiffSpineNode<'t> {
    // Hash the syntax trees and compute their weights
    let mut origin_weighted_tree = weight_tree(origin_tree);
    origin_weighted_tree.weight += 1; // Small incentive to keep the root node
    let modified_weighted_tree = weight_tree(modified_tree);

    // Merge the common parts from both trees to create a spine of unchanged
    // structure.
    let aligned_tree = align_trees(origin_weighted_tree, modified_weighted_tree);

    // Compute the difference as a deletion and an insertion tree by eliding
    // parts reused from original to modified
    find_metavariable_elisions(&aligned_tree, kind_whitelist)
}
