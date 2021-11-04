mod alignment;
mod elision;
mod hash;
mod tree;

pub use tree::{ChangeNode, Metavariable, SpineNode, SpineSeqNode};

use crate::syn_tree::SynNode;
use alignment::align_trees;
use elision::{find_metavariable_elisions, reduce_weight_on_elision_sites};
use hash::hash_tree;

pub fn compute_diff<'t>(origin_tree: &SynNode<'t>, modified_tree: &SynNode<'t>) -> SpineNode<'t> {
    // Hash the syntax trees and compute their weights
    let mut origin_hashed_tree = hash_tree(origin_tree);
    origin_hashed_tree.weight += 1; // Small incentive to keep the root node
    let modified_hashed_tree = hash_tree(modified_tree);

    let (origin_weighted_tree, modified_weighted_tree) =
        reduce_weight_on_elision_sites(origin_hashed_tree, modified_hashed_tree);

    // Merge the common parts from both trees to create a spine of unchanged
    // structure.
    let aligned_tree = align_trees(origin_weighted_tree, modified_weighted_tree);

    // Compute the difference as a deletion and an insertion tree by eliding
    // parts reused from original to modified
    find_metavariable_elisions(&aligned_tree)
}
