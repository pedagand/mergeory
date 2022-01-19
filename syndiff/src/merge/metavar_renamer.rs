use super::colors::{ColoredChangeNode, ColoredSpineNode, ColoredSpineSeqNode};
use super::{DelNode, InsNode, MergedInsNode, MergedSpineNode, MergedSpineSeqNode};
use crate::Metavariable;

pub struct MetavarRenamer {
    new_metavars: Vec<Option<Metavariable>>,
    next_metavar: usize,
}

impl MetavarRenamer {
    fn rename(&mut self, mv: Metavariable) -> Metavariable {
        if self.new_metavars.len() <= mv.0 {
            self.new_metavars.resize_with(mv.0 + 1, Default::default)
        }
        *self.new_metavars[mv.0].get_or_insert_with(|| {
            let mv = self.next_metavar;
            self.next_metavar += 1;
            Metavariable(mv)
        })
    }
}

fn rename_metavars_in_change(change: &mut ColoredChangeNode, renamer: &mut MetavarRenamer) {
    match change {
        ColoredChangeNode::InPlace(change) => change
            .data
            .visit_mut(|sub| rename_metavars_in_change(&mut sub.node, renamer)),
        ColoredChangeNode::Elided(mv) => mv.data = renamer.rename(mv.data),
    }
}

fn rename_metavars_in_diff_spine(spine: &mut ColoredSpineNode, renamer: &mut MetavarRenamer) {
    match spine {
        ColoredSpineNode::Spine(spine) => {
            spine.visit_mut(|sub| rename_metavars_in_diff_spine_subtree(sub, renamer))
        }
        ColoredSpineNode::Unchanged => (),
        ColoredSpineNode::Changed(del, ins) => {
            rename_metavars_in_change(del, renamer);
            rename_metavars_in_change(ins, renamer);
        }
    }
}

fn rename_metavars_in_diff_spine_subtree(
    subtree: &mut ColoredSpineSeqNode,
    renamer: &mut MetavarRenamer,
) {
    match subtree {
        ColoredSpineSeqNode::Zipped(spine) => {
            rename_metavars_in_diff_spine(&mut spine.node, renamer)
        }
        ColoredSpineSeqNode::Deleted(del_list) => {
            for del in del_list {
                rename_metavars_in_change(&mut del.node, renamer);
            }
        }
        ColoredSpineSeqNode::Inserted(ins_list) => {
            for ins in ins_list {
                rename_metavars_in_change(&mut ins.node, renamer);
            }
        }
    }
}

fn rename_metavars_in_del(del: &mut DelNode, renamer: &mut MetavarRenamer) {
    match del {
        DelNode::InPlace(del) => del
            .data
            .visit_mut(|sub| rename_metavars_in_del(&mut sub.node, renamer)),
        DelNode::Elided(mv) => mv.data = renamer.rename(mv.data),
        DelNode::MetavariableConflict(mv, del, repl) => {
            *mv = renamer.rename(*mv);
            rename_metavars_in_del(del, renamer);
            for before_ins in &mut repl.ins_before {
                rename_metavars_in_ins(before_ins, renamer);
            }
            if let Some(repl) = &mut repl.self_repl {
                rename_metavars_in_ins(repl, renamer);
            }
            for after_ins in &mut repl.ins_after {
                rename_metavars_in_ins(after_ins, renamer);
            }
        }
    }
}

fn rename_metavars_in_ins(ins: &mut InsNode, renamer: &mut MetavarRenamer) {
    match ins {
        InsNode::InPlace(ins) => ins
            .data
            .visit_mut(|sub| rename_metavars_in_ins(&mut sub.node, renamer)),
        InsNode::Elided(mv) => mv.data = renamer.rename(mv.data),
        InsNode::Inlined(ins_repl) => {
            for ins in &mut ins_repl.data {
                rename_metavars_in_ins(ins, renamer)
            }
        }
    }
}

fn rename_metavars_in_merged_ins(ins: &mut MergedInsNode, renamer: &mut MetavarRenamer) {
    match ins {
        MergedInsNode::InPlace(ins) => {
            ins.visit_mut(|sub| rename_metavars_in_merged_ins(&mut sub.node, renamer))
        }
        MergedInsNode::Elided(mv) => *mv = renamer.rename(*mv),
        MergedInsNode::SingleIns(ins) => rename_metavars_in_ins(ins, renamer),
        MergedInsNode::Conflict(left_ins, right_ins) => {
            rename_metavars_in_ins(left_ins, renamer);
            rename_metavars_in_ins(right_ins, renamer);
        }
    }
}

fn rename_metavars_in_merged_spine(spine: &mut MergedSpineNode, renamer: &mut MetavarRenamer) {
    match spine {
        MergedSpineNode::Spine(spine) => {
            spine.visit_mut(|sub| rename_metavars_in_merged_spine_subtree(sub, renamer))
        }
        MergedSpineNode::Unchanged => (),
        MergedSpineNode::Changed(del, ins) => {
            rename_metavars_in_del(del, renamer);
            rename_metavars_in_merged_ins(ins, renamer);
        }
    }
}

fn rename_metavars_in_merged_spine_subtree(
    subtree: &mut MergedSpineSeqNode,
    renamer: &mut MetavarRenamer,
) {
    match subtree {
        MergedSpineSeqNode::Zipped(spine) => {
            rename_metavars_in_merged_spine(&mut spine.node, renamer)
        }
        MergedSpineSeqNode::Deleted(del_list) => {
            for del in del_list {
                rename_metavars_in_del(&mut del.node, renamer);
            }
        }
        MergedSpineSeqNode::DeleteConflict(_, del, ins) => {
            rename_metavars_in_del(del, renamer);
            rename_metavars_in_ins(ins, renamer);
        }
        MergedSpineSeqNode::Inserted(ins_list) => {
            for ins in ins_list {
                rename_metavars_in_ins(&mut ins.node, renamer);
            }
        }
        MergedSpineSeqNode::InsertOrderConflict(left_ins_list, right_ins_list) => {
            for ins_list in [left_ins_list, right_ins_list] {
                for ins in ins_list {
                    rename_metavars_in_ins(&mut ins.node, renamer);
                }
            }
        }
    }
}

pub fn rename_metavars(input: &mut ColoredSpineNode, first_metavar: usize) -> usize {
    let mut renamer = MetavarRenamer {
        new_metavars: Vec::new(),
        next_metavar: first_metavar,
    };
    rename_metavars_in_diff_spine(input, &mut renamer);
    renamer.next_metavar
}

pub fn canonicalize_metavars(input: &mut MergedSpineNode) {
    let mut renamer = MetavarRenamer {
        new_metavars: Vec::new(),
        next_metavar: 0,
    };
    rename_metavars_in_merged_spine(input, &mut renamer);
}
