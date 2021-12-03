//! Merge insertion & deletion trees if they are identical modulo colors

use super::{Colored, DelNode, InsNode, InsSeqNode};
use crate::generic_tree::{Subtree, Tree};

pub fn merge_id_ins<'t>(left: &InsNode<'t>, right: &InsNode<'t>) -> Option<InsNode<'t>> {
    match (left, right) {
        (InsNode::InPlace(left), InsNode::InPlace(right)) => Some(InsNode::InPlace(
            Colored::merge(left.as_ref(), right.as_ref(), |l, r| {
                Tree::merge_to(l, r, merge_id_ins_seq)
            })?,
        )),
        (InsNode::Elided(left), InsNode::Elided(right)) if left == right => {
            Some(InsNode::Elided(*left))
        }
        (InsNode::Conflict(left_confl), InsNode::Conflict(right_confl))
            if left_confl.len() == right_confl.len() =>
        {
            Some(InsNode::Conflict(
                left_confl
                    .iter()
                    .zip(right_confl)
                    .map(|(l, r)| merge_id_ins(l, r))
                    .collect::<Option<_>>()?,
            ))
        }
        _ => None,
    }
}

fn merge_id_ins_seq<'t>(
    left: &[InsSeqNode<'t>],
    right: &[InsSeqNode<'t>],
) -> Option<Vec<InsSeqNode<'t>>> {
    if left.len() != right.len() {
        return None;
    }

    left.iter()
        .zip(right)
        .map(|pair| match pair {
            (InsSeqNode::Node(left), InsSeqNode::Node(right)) => Some(InsSeqNode::Node(
                Subtree::merge(left.as_ref(), right.as_ref(), merge_id_ins)?,
            )),
            (InsSeqNode::DeleteConflict(left), InsSeqNode::DeleteConflict(right)) => {
                Some(InsSeqNode::DeleteConflict(Subtree::merge(
                    left.as_ref(),
                    right.as_ref(),
                    merge_id_ins,
                )?))
            }
            (InsSeqNode::InsertOrderConflict(left), InsSeqNode::InsertOrderConflict(right))
                if left.len() == right.len() =>
            {
                Some(InsSeqNode::InsertOrderConflict(
                    left.iter()
                        .zip(right)
                        .map(|(left, right)| {
                            Colored::merge(left.as_ref(), right.as_ref(), |left, right| {
                                if left.len() != right.len() {
                                    return None;
                                }
                                left.iter()
                                    .zip(right)
                                    .map(|(left, right)| {
                                        Subtree::merge(left.as_ref(), right.as_ref(), merge_id_ins)
                                    })
                                    .collect()
                            })
                        })
                        .collect::<Option<_>>()?,
                ))
            }
            _ => None,
        })
        .collect()
}

pub fn is_del_equivalent_to_ins(del: &DelNode, ins: &InsNode) -> bool {
    match (del, ins) {
        (DelNode::InPlace(del), InsNode::InPlace(ins)) => {
            Tree::compare(&del.data, &ins.data, is_del_equivalent_to_ins_seq)
        }
        (DelNode::Elided(del_mv), InsNode::Elided(ins_mv)) => del_mv.data == *ins_mv,
        (DelNode::MetavariableConflict(_, del, _), ins) => is_del_equivalent_to_ins(del, ins),
        _ => false,
    }
}

fn is_del_equivalent_to_ins_seq(del_seq: &[Subtree<DelNode>], ins_seq: &[InsSeqNode]) -> bool {
    if del_seq.len() != ins_seq.len() {
        return false;
    }
    del_seq.iter().zip(ins_seq).all(|(del, ins)| match ins {
        InsSeqNode::Node(ins) => Subtree::compare(del, ins, is_del_equivalent_to_ins),
        _ => false,
    })
}
