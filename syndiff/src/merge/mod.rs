mod align_spine;
mod colors;
mod conflict_counter;
mod id_merger;
mod merge_del;
mod merge_ins;
mod metavar_remover;
mod metavar_renamer;
mod patch;
mod subst;
mod tree;

pub use colors::{Color, ColorSet, Colored};
pub use conflict_counter::count_conflicts;
pub use metavar_remover::remove_metavars;
pub use metavar_renamer::canonicalize_metavars;
pub use patch::apply_patch;
pub use tree::{DelNode, InsNode, InsSeqNode, MetavarInsReplacement, SpineNode, SpineSeqNode};

use crate::DiffSpineNode;
use align_spine::align_spines;
use merge_del::merge_del;
use merge_ins::merge_ins;
use metavar_renamer::rename_metavars;
use subst::apply_metavar_substitutions;

pub fn merge_multi_diffs<'t>(
    mut left: SpineNode<'t>,
    mut right: SpineNode<'t>,
) -> Option<SpineNode<'t>> {
    let left_end_mv = rename_metavars(&mut left, 0);
    let right_end_mv = rename_metavars(&mut right, left_end_mv);
    let (aligned, nb_metavars) = align_spines(left, right, right_end_mv)?;

    let (ins_merged, ins_subst) = merge_ins(aligned, nb_metavars);
    let (mut merged, del_subst) = merge_del(ins_merged, nb_metavars)?;
    apply_metavar_substitutions(&mut merged, del_subst, ins_subst);

    Some(merged)
}

pub fn merge_diffs<'t>(diffs: &[DiffSpineNode<'t>]) -> Option<SpineNode<'t>> {
    let mut diff_iter = diffs.iter().enumerate();
    let first_multi_diff = SpineNode::with_color(diff_iter.next()?.1, 0.try_into().unwrap());
    let mut merged_diff = diff_iter.try_fold(first_multi_diff, |diff_acc, (i, next_diff)| {
        merge_multi_diffs(
            diff_acc,
            SpineNode::with_color(next_diff, i.try_into().unwrap()),
        )
    })?;
    canonicalize_metavars(&mut merged_diff);
    Some(merged_diff)
}
