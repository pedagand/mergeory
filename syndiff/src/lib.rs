mod family_traits;

mod elided_tree;
mod hash_tree;
mod spine_tree;
mod weighted_tree;

pub mod ast;
pub mod diff_tree;
pub mod multi_diff_tree;
pub mod source_repr;
pub mod token_trees;

use crate::diff_tree::name_metavariables;
use crate::elided_tree::{elide_tree_with, find_wanted_elisions};
use crate::hash_tree::{hash_tree, tables_intersection};
use crate::spine_tree::zip_spine;
use crate::weighted_tree::compute_weight;

pub fn compute_diff(original: syn::File, modified: syn::File) -> ast::diff::File {
    // Parse both input files and hash their AST
    let (origin_hash_ast, origin_hash_tables) = hash_tree(original);
    let (modified_hash_ast, modified_hash_tables) = hash_tree(modified);

    // Find the common subtrees between original and modified versions
    let unified_hash_tables = tables_intersection(origin_hash_tables, modified_hash_tables);

    // Find which of the common subtrees will actually be elided in both trees.
    // This avoids elided part appearing only inside one of the subtrees.
    let origin_wanted_elisions = find_wanted_elisions(&origin_hash_ast, &unified_hash_tables);
    let modified_wanted_elisions = find_wanted_elisions(&modified_hash_ast, &unified_hash_tables);
    let elision_tables = tables_intersection(origin_wanted_elisions, modified_wanted_elisions);

    // Remove any reference to subtrees inside unified_hash_tables such that either:
    // * The subtree must be elided
    // * The Rc counter of the subtree is 1
    drop(unified_hash_tables);

    // Compute the difference as a deletion and an insertion tree by eliding
    // parts reused from original to modified
    let deletion_ast = elide_tree_with(origin_hash_ast, &elision_tables);
    let insertion_ast = elide_tree_with(modified_hash_ast, &elision_tables);

    // Merge the deletion and the insertion tree on their common part to create
    // a spine of unchanged structure.
    let weighted_del: ast::weighted::File = compute_weight(deletion_ast);
    let weighted_ins: ast::weighted::File = compute_weight(insertion_ast);
    let spine_ast = zip_spine(weighted_del, weighted_ins).unwrap();
    name_metavariables(spine_ast)
}

use crate::multi_diff_tree::{canonicalize_metavars, merge_multi_diffs, with_color};

pub fn merge_diffs(diffs: Vec<ast::diff::File>) -> Option<ast::multi_diff::File> {
    let mut diff_iter = diffs.into_iter().enumerate();
    let first_multi_diff = with_color(diff_iter.next()?.1, 0);
    let mut merged_diff = diff_iter.try_fold(first_multi_diff, |diff_acc, (i, next_diff)| {
        merge_multi_diffs(diff_acc, with_color(next_diff, i))
    })?;
    canonicalize_metavars(&mut merged_diff);
    Some(merged_diff)
}
