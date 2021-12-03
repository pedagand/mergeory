use super::{DelNode, InsNode, InsSeqNode, MetavarInsReplacement, SpineNode, SpineSeqNode};
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

fn rename_metavars_in_del(del: &mut DelNode, renamer: &mut MetavarRenamer) {
    match del {
        DelNode::InPlace(del) => del
            .data
            .visit_mut(|sub| rename_metavars_in_del(&mut sub.node, renamer)),
        DelNode::Elided(mv) => mv.data = renamer.rename(mv.data),
        DelNode::MetavariableConflict(mv, del, ins) => {
            *mv = renamer.rename(*mv);
            rename_metavars_in_del(del, renamer);
            if let MetavarInsReplacement::Inlined(ins) = ins {
                rename_metavars_in_ins(ins, renamer);
            }
        }
    }
}

fn rename_metavars_in_ins(ins: &mut InsNode, renamer: &mut MetavarRenamer) {
    match ins {
        InsNode::InPlace(ins) => ins
            .data
            .visit_mut(|sub| rename_metavars_in_ins_subtree(sub, renamer)),
        InsNode::Elided(mv) => *mv = renamer.rename(*mv),
        InsNode::Conflict(ins_list) => {
            for ins in ins_list {
                rename_metavars_in_ins(ins, renamer);
            }
        }
    }
}

fn rename_metavars_in_ins_subtree(ins: &mut InsSeqNode, renamer: &mut MetavarRenamer) {
    match ins {
        InsSeqNode::Node(node) => rename_metavars_in_ins(&mut node.node, renamer),
        InsSeqNode::DeleteConflict(ins) => rename_metavars_in_ins(&mut ins.node, renamer),
        InsSeqNode::InsertOrderConflict(conflicts) => {
            for ins_list in conflicts {
                for ins in &mut ins_list.data {
                    rename_metavars_in_ins(&mut ins.node, renamer);
                }
            }
        }
    }
}

fn rename_metavars_in_spine(spine: &mut SpineNode, renamer: &mut MetavarRenamer) {
    match spine {
        SpineNode::Spine(spine) => {
            spine.visit_mut(|sub| rename_metavars_in_spine_subtree(sub, renamer))
        }
        SpineNode::Unchanged => (),
        SpineNode::Changed(del, ins) => {
            rename_metavars_in_del(del, renamer);
            rename_metavars_in_ins(ins, renamer);
        }
    }
}

fn rename_metavars_in_spine_subtree(subtree: &mut SpineSeqNode, renamer: &mut MetavarRenamer) {
    match subtree {
        SpineSeqNode::Zipped(spine) => rename_metavars_in_spine(&mut spine.node, renamer),
        SpineSeqNode::Deleted(del_list) => {
            for del in del_list {
                rename_metavars_in_del(&mut del.node, renamer);
            }
        }
        SpineSeqNode::DeleteConflict(_, del, ins) => {
            rename_metavars_in_del(del, renamer);
            rename_metavars_in_ins(ins, renamer);
        }
        SpineSeqNode::Inserted(ins_list) => {
            for ins in &mut ins_list.data {
                rename_metavars_in_ins(&mut ins.node, renamer);
            }
        }
        SpineSeqNode::InsertOrderConflict(conflicts) => {
            for ins_list in conflicts {
                for ins in &mut ins_list.data {
                    rename_metavars_in_ins(&mut ins.node, renamer);
                }
            }
        }
    }
}

pub fn rename_metavars(input: &mut SpineNode, first_metavar: usize) -> usize {
    let mut renamer = MetavarRenamer {
        new_metavars: Vec::new(),
        next_metavar: first_metavar,
    };
    rename_metavars_in_spine(input, &mut renamer);
    renamer.next_metavar
}

pub fn canonicalize_metavars(input: &mut SpineNode) {
    rename_metavars(input, 0);
}
