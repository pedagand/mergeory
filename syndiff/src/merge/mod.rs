mod align_spine;
mod conflict_counter;
mod merge_del;
mod merge_ins;
mod metavar_remover;
mod metavar_renamer;
mod patch;
mod subst;
mod tree;

pub use conflict_counter::count_conflicts;
pub use metavar_remover::remove_metavars;
pub use metavar_renamer::canonicalize_metavars;
pub use patch::apply_patch;
pub use tree::{
    DelNode, InsNode, MergedInsNode, MergedSpineNode, MergedSpineSeqNode, MetavarInsReplacement,
};

use crate::DiffSpineNode;
use align_spine::align_spines;
use merge_del::merge_del;
use merge_ins::merge_ins;
use metavar_renamer::rename_metavars;
use subst::apply_metavar_substitutions;

pub fn merge_diffs<'t>(
    mut left: DiffSpineNode<'t>,
    mut right: DiffSpineNode<'t>,
) -> Option<MergedSpineNode<'t>> {
    let left_end_mv = rename_metavars(&mut left, 0);
    let right_end_mv = rename_metavars(&mut right, left_end_mv);
    let (aligned, nb_metavars) = align_spines(left, right, right_end_mv)?;

    let (ins_merged, ins_subst) = merge_ins(aligned, nb_metavars);
    let (mut merged, del_subst) = merge_del(ins_merged, nb_metavars)?;
    apply_metavar_substitutions(&mut merged, del_subst, ins_subst);

    Some(merged)
}
