mod family_traits;

mod ellided_tree;
mod hash_tree;
mod spine_tree;
mod weighted_tree;

pub mod ast;
pub mod diff_tree;
pub mod source_repr;

use crate::diff_tree::name_metavariables;
use crate::ellided_tree::{ellide_tree_with, find_wanted_ellisions};
use crate::hash_tree::{hash_tree, tables_intersection};
use crate::spine_tree::zip_spine;
use crate::weighted_tree::compute_weight;

pub fn compute_diff(original: syn::File, modified: syn::File) -> ast::diff::File {
    // Parse both input files and hash their AST
    let (origin_hash_ast, origin_hash_tables) = hash_tree(original);
    let (modified_hash_ast, modified_hash_tables) = hash_tree(modified);

    // Find the common subtrees between original and modified versions
    let unified_hash_tables = tables_intersection(origin_hash_tables, modified_hash_tables);

    // Find which of the common subtrees will actually be ellided in both trees.
    // This avoids ellided part appearing only inside one of the subtrees.
    let origin_wanted_ellisions = find_wanted_ellisions(&origin_hash_ast, &unified_hash_tables);
    let modified_wanted_ellisions = find_wanted_ellisions(&modified_hash_ast, &unified_hash_tables);
    let ellision_tables = tables_intersection(origin_wanted_ellisions, modified_wanted_ellisions);

    // Remove any reference to subtrees inside unified_hash_tables such that either:
    // * The subtree must be ellided
    // * The Rc counter of the subtree is 1
    drop(unified_hash_tables);

    // Compute the difference as a deletion and an insertion tree by elliding
    // parts reused from original to modified
    let deletion_ast = ellide_tree_with(origin_hash_ast, &ellision_tables);
    let insertion_ast = ellide_tree_with(modified_hash_ast, &ellision_tables);

    // Merge the deletion and the insertion tree on their common part to create
    // a spine of unchanged structure.
    let weighted_del: ast::weighted::File = compute_weight(deletion_ast);
    let weighted_ins: ast::weighted::File = compute_weight(insertion_ast);
    let spine_ast = zip_spine(weighted_del, weighted_ins).unwrap();
    name_metavariables(spine_ast)
}
