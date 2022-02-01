use super::align_spine::{AlignedSpineNode, AlignedSpineSeqNode, InsSpineNode, InsSpineSeqNode};
use super::colors::{Colored, ColoredChangeNode as ChangeNode};
use super::{DelNode, InsNode, MergedInsNode, MetavarInsReplacement};
use crate::generic_tree::{FieldId, Subtree, Tree};

pub enum InsMergedSpineNode<'t> {
    Spine(Tree<'t, InsMergedSpineSeqNode<'t>>),
    Unchanged,
    OneChange(DelNode<'t>, MergedInsNode<'t>),
    BothChanged(DelNode<'t>, DelNode<'t>, MergedInsNode<'t>),
}
pub enum InsMergedSpineSeqNode<'t> {
    Zipped(Subtree<InsMergedSpineNode<'t>>),
    BothDeleted(Option<FieldId>, DelNode<'t>, DelNode<'t>),
    DeleteConflict(Option<FieldId>, DelNode<'t>, DelNode<'t>, InsNode<'t>),
    Inserted(Vec<Subtree<InsNode<'t>>>),
    InsertOrderConflict(Vec<Subtree<InsNode<'t>>>, Vec<Subtree<InsNode<'t>>>),
}

pub type MetavarInsReplacementList<'t> = Vec<MetavarInsReplacement<'t>>;

fn merge_ins_nodes<'t>(left: InsNode<'t>, right: InsNode<'t>) -> MergedInsNode<'t> {
    match (left, right) {
        (InsNode::InPlace(left_node), InsNode::InPlace(right_node))
            if Tree::compare_subtrees(&left_node.data, &right_node.data, |_, _| true) =>
        {
            MergedInsNode::InPlace(
                Tree::merge_subtrees_into(left_node.data, right_node.data, |l, r| {
                    Some(merge_ins_nodes(l, r))
                })
                .unwrap(),
            )
        }
        (left, right) => MergedInsNode::Conflict(left, right),
    }
}

fn flatten_ins_spine(ins_spine: InsSpineNode) -> InsNode {
    match ins_spine {
        InsSpineNode::Spine(ins_subtree) => InsNode::InPlace(Colored::new_white(
            ins_subtree.convert_into(flatten_ins_spine_seq),
        )),
        InsSpineNode::Unchanged(mv) => InsNode::Elided(Colored::new_white(mv)),
        InsSpineNode::Changed(ins) => ins.into(),
    }
}

fn flatten_ins_spine_seq(ins_spine_seq: Vec<InsSpineSeqNode>) -> Vec<Subtree<InsNode>> {
    let mut ins_seq = Vec::new();
    for ins_spine_seq_node in ins_spine_seq {
        match ins_spine_seq_node {
            InsSpineSeqNode::Zipped(ins_subtree) => {
                ins_seq.push(ins_subtree.map(flatten_ins_spine))
            }
            InsSpineSeqNode::Deleted => (),
            InsSpineSeqNode::Inserted(ins_list) => {
                for ins in ins_list {
                    ins_seq.push(ins.map(InsNode::from))
                }
            }
        }
    }
    ins_seq
}

fn can_inline_ins_in_del(
    ins: &InsSpineNode,
    del: &ChangeNode,
    allow_nested_deletions: bool,
) -> bool {
    match (ins, del) {
        (InsSpineNode::Unchanged(_), _) => true,
        (_, ChangeNode::Elided(_)) => true,
        (InsSpineNode::Spine(ins_subtree), ChangeNode::InPlace(del_subtree)) => {
            Tree::compare(ins_subtree, &del_subtree.data, |i, d| {
                can_inline_ins_seq_in_del(i, d, allow_nested_deletions)
            })
        }
        (InsSpineNode::Changed(_), ChangeNode::InPlace(_)) => {
            // Change occured and it is not facing a metavariable: refuse inlining
            false
        }
    }
}

fn can_inline_ins_seq_in_del(
    ins_seq: &[InsSpineSeqNode],
    del_seq: &[Subtree<ChangeNode>],
    allow_nested_deletions: bool,
) -> bool {
    if ins_seq.len() != del_seq.len() {
        return false;
    }

    ins_seq
        .iter()
        .zip(del_seq)
        .all(|(ins_spine_seq_node, del)| match ins_spine_seq_node {
            InsSpineSeqNode::Zipped(ins) if del.field == ins.field => {
                can_inline_ins_in_del(&ins.node, &del.node, allow_nested_deletions)
            }
            InsSpineSeqNode::Deleted => allow_nested_deletions,
            _ => false,
        })
}

fn inline_ins_in_del<'t>(
    ins: InsSpineNode<'t>,
    del: ChangeNode<'t>,
    metavars_status: &mut [MetavarInsReplacementList<'t>],
) -> DelNode<'t> {
    match (ins, del) {
        (InsSpineNode::Unchanged(_), del) => register_kept_metavars(del, metavars_status),
        (ins_spine, ChangeNode::Elided(mv)) => {
            // Here we must clone the insert tree once to check for potential conflicts
            let ins = flatten_ins_spine(ins_spine);
            metavars_status[mv.data.0].push(MetavarInsReplacement::Inlined(ins.clone()));
            DelNode::MetavariableConflict(
                mv.data,
                Box::new(DelNode::Elided(mv)),
                MetavarInsReplacement::Inlined(ins),
            )
        }
        (InsSpineNode::Spine(ins_subtree), ChangeNode::InPlace(del_subtree)) => {
            DelNode::InPlace(Colored {
                data: Tree::merge_into(ins_subtree, del_subtree.data, |ins, del| {
                    inline_ins_seq_in_del(ins, del, metavars_status)
                })
                .unwrap(),
                color: del_subtree.color,
            })
        }
        (InsSpineNode::Changed(_), ChangeNode::InPlace(_)) => {
            panic!("inline_ins_in_del called with incompatible trees")
        }
    }
}

fn inline_ins_seq_in_del<'t>(
    ins_spine_seq: Vec<InsSpineSeqNode<'t>>,
    del_seq: Vec<Subtree<ChangeNode<'t>>>,
    metavars_status: &mut [MetavarInsReplacementList<'t>],
) -> Option<Vec<Subtree<DelNode<'t>>>> {
    Some(
        ins_spine_seq
            .into_iter()
            .zip(del_seq)
            .map(|(ins_spine_seq_node, del)| match ins_spine_seq_node {
                InsSpineSeqNode::Zipped(ins_spine) => {
                    del.map(|del| inline_ins_in_del(ins_spine.node, del, metavars_status))
                }
                InsSpineSeqNode::Deleted => {
                    del.map(|del| register_kept_metavars(del, metavars_status))
                }
                _ => panic!("InsSpineSeq cannot be inlined in the deletion tree"),
            })
            .collect(),
    )
}

fn register_kept_metavars<'t>(
    del: ChangeNode<'t>,
    metavars_status: &mut [MetavarInsReplacementList<'t>],
) -> DelNode<'t> {
    match del {
        ChangeNode::InPlace(del_subtree) => {
            DelNode::InPlace(del_subtree.map(|del| {
                del.map_subtrees_into(|sub| register_kept_metavars(sub, metavars_status))
            }))
        }
        ChangeNode::Elided(mv) => {
            metavars_status[mv.data.0].push(MetavarInsReplacement::InferFromDel);
            DelNode::MetavariableConflict(
                mv.data,
                Box::new(DelNode::Elided(mv)),
                MetavarInsReplacement::InferFromDel,
            )
        }
    }
}

fn merge_ins_in_spine<'t>(
    node: AlignedSpineNode<'t>,
    metavars_status: &mut [MetavarInsReplacementList<'t>],
    allow_nested_deletions: bool,
) -> InsMergedSpineNode<'t> {
    match node {
        AlignedSpineNode::Spine(spine) => {
            InsMergedSpineNode::Spine(spine.map_children_into(|ch| {
                merge_ins_in_spine_seq_node(ch, metavars_status, allow_nested_deletions)
            }))
        }
        AlignedSpineNode::Unchanged => InsMergedSpineNode::Unchanged,
        AlignedSpineNode::OneChange(del, ins) => InsMergedSpineNode::OneChange(
            register_kept_metavars(del, metavars_status),
            MergedInsNode::SingleIns(ins.into()),
        ),
        AlignedSpineNode::BothChanged(left_del, left_ins, right_del, right_ins) => {
            match (
                can_inline_ins_in_del(&left_ins, &right_del, allow_nested_deletions),
                can_inline_ins_in_del(&right_ins, &left_del, allow_nested_deletions),
            ) {
                (true, true) | (false, false) => InsMergedSpineNode::BothChanged(
                    register_kept_metavars(left_del, metavars_status),
                    register_kept_metavars(right_del, metavars_status),
                    merge_ins_nodes(flatten_ins_spine(left_ins), flatten_ins_spine(right_ins)),
                ),
                (true, false) => InsMergedSpineNode::BothChanged(
                    inline_ins_in_del(left_ins, right_del, metavars_status),
                    register_kept_metavars(left_del, metavars_status),
                    MergedInsNode::SingleIns(flatten_ins_spine(right_ins)),
                ),
                (false, true) => InsMergedSpineNode::BothChanged(
                    inline_ins_in_del(right_ins, left_del, metavars_status),
                    register_kept_metavars(right_del, metavars_status),
                    MergedInsNode::SingleIns(flatten_ins_spine(left_ins)),
                ),
            }
        }
    }
}

fn merge_ins_in_spine_seq_node<'t>(
    seq_node: AlignedSpineSeqNode<'t>,
    metavars_status: &mut [MetavarInsReplacementList<'t>],
    allow_nested_deletions: bool,
) -> InsMergedSpineSeqNode<'t> {
    match seq_node {
        AlignedSpineSeqNode::Zipped(node) => InsMergedSpineSeqNode::Zipped(
            node.map(|node| merge_ins_in_spine(node, metavars_status, allow_nested_deletions)),
        ),
        AlignedSpineSeqNode::BothDeleted(field, left_del, right_del) => {
            InsMergedSpineSeqNode::BothDeleted(
                field,
                register_kept_metavars(left_del, metavars_status),
                register_kept_metavars(right_del, metavars_status),
            )
        }
        AlignedSpineSeqNode::DeleteConflict(field, del, conflict_del, conflict_ins) => {
            if can_inline_ins_in_del(&conflict_ins, &del, allow_nested_deletions) {
                InsMergedSpineSeqNode::BothDeleted(
                    field,
                    inline_ins_in_del(conflict_ins, del, metavars_status),
                    register_kept_metavars(conflict_del, metavars_status),
                )
            } else {
                InsMergedSpineSeqNode::DeleteConflict(
                    field,
                    register_kept_metavars(del, metavars_status),
                    register_kept_metavars(conflict_del, metavars_status),
                    flatten_ins_spine(conflict_ins),
                )
            }
        }
        AlignedSpineSeqNode::Inserted(ins_vec) => InsMergedSpineSeqNode::Inserted(
            ins_vec
                .into_iter()
                .map(|sub| sub.map(InsNode::from))
                .collect(),
        ),
        AlignedSpineSeqNode::InsertOrderConflict(left, right) => {
            InsMergedSpineSeqNode::InsertOrderConflict(
                left.into_iter().map(|sub| sub.map(InsNode::from)).collect(),
                right
                    .into_iter()
                    .map(|sub| sub.map(InsNode::from))
                    .collect(),
            )
        }
    }
}

pub fn merge_ins(
    input: AlignedSpineNode,
    nb_vars: usize,
    allow_nested_deletions: bool,
) -> (InsMergedSpineNode, Vec<MetavarInsReplacementList>) {
    let mut metavars_status = Vec::new();
    metavars_status.resize_with(nb_vars, Vec::new);
    let output = merge_ins_in_spine(input, &mut metavars_status, allow_nested_deletions);
    (output, metavars_status)
}
