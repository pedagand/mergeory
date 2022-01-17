mod align_spine;
mod colors;
mod conflict_counter;
mod merge_del;
mod merge_ins;
mod metavar_remover;
mod metavar_renamer;
mod patch;
mod subst;
mod tree;

pub use colors::{Color, Colored, ColoredSpineNode};
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

#[derive(Default)]
pub struct MergeOptions {
    pub allow_nested_deletions: bool,
    pub ordered_insertions: bool,
}

pub fn merge_diffs<'t>(
    left: &DiffSpineNode<'t>,
    right: &DiffSpineNode<'t>,
    options: MergeOptions,
) -> Option<MergedSpineNode<'t>> {
    let mut left = ColoredSpineNode::with_color(left, Color::Left);
    let mut right = ColoredSpineNode::with_color(right, Color::Right);
    let left_end_mv = rename_metavars(&mut left, 0);
    let right_end_mv = rename_metavars(&mut right, left_end_mv);
    let (aligned, nb_metavars) = align_spines(left, right, right_end_mv)?;

    let (ins_merged, ins_subst) = merge_ins(aligned, nb_metavars, options.allow_nested_deletions);
    let (mut merged, del_subst) = merge_del(ins_merged, nb_metavars)?;
    apply_metavar_substitutions(
        &mut merged,
        del_subst,
        ins_subst,
        options.ordered_insertions,
    );

    Some(merged)
}
