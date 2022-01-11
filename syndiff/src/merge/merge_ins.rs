use super::align_spine::{AlignedSpineNode, AlignedSpineSeqNode, InsSpineNode, InsSpineSeqNode};
use super::{ColorSet, Colored, DelNode, InsNode, InsSeqNode, MetavarInsReplacement};
use crate::generic_tree::{FieldId, Subtree, Tree};

pub enum InsMergedSpineNode<'t> {
    Spine(Tree<'t, InsMergedSpineSeqNode<'t>>),
    Unchanged,
    OneChange(DelNode<'t>, InsNode<'t>),
    BothChanged(DelNode<'t>, DelNode<'t>, InsNode<'t>),
}
pub enum InsMergedSpineSeqNode<'t> {
    Zipped(Subtree<InsMergedSpineNode<'t>>),
    BothDeleted(Option<FieldId>, DelNode<'t>, DelNode<'t>),
    DeleteConflict(Option<FieldId>, DelNode<'t>, DelNode<'t>, InsNode<'t>),
    Inserted(Vec<Colored<Vec<Subtree<InsNode<'t>>>>>),
}

pub type MetavarInsReplacementList<'t> = Vec<MetavarInsReplacement<'t>>;

fn merge_ins_nodes<'t>(left: InsNode<'t>, right: InsNode<'t>) -> InsNode<'t> {
    match (left, right) {
        (InsNode::InPlace(left_node), InsNode::InPlace(right_node))
            if Tree::compare(&left_node.data, &right_node.data, can_merge_ins_seq) =>
        {
            InsNode::InPlace(
                Colored::merge(left_node, right_node, |l, r| {
                    Tree::merge_into(l, r, merge_ins_seq)
                })
                .unwrap(),
            )
        }
        (InsNode::Conflict(mut conflict), InsNode::Conflict(other_conflict)) => {
            conflict.extend(other_conflict);
            InsNode::Conflict(conflict)
        }
        (InsNode::Conflict(mut conflict), other) | (other, InsNode::Conflict(mut conflict)) => {
            conflict.push(other);
            InsNode::Conflict(conflict)
        }
        (left, right) => InsNode::Conflict(vec![left, right]),
    }
}

fn can_merge_ins_seq(left: &[InsSeqNode], right: &[InsSeqNode]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter().zip(right).all(|pair| match pair {
        (
            InsSeqNode::Node(left_sub) | InsSeqNode::DeleteConflict(left_sub),
            InsSeqNode::Node(right_sub) | InsSeqNode::DeleteConflict(right_sub),
        ) => left_sub.field == right_sub.field,
        (InsSeqNode::InsertOrderConflict(_), _) | (_, InsSeqNode::InsertOrderConflict(_)) => false,
    })
}

fn merge_ins_seq<'t>(
    left: Vec<InsSeqNode<'t>>,
    right: Vec<InsSeqNode<'t>>,
) -> Option<Vec<InsSeqNode<'t>>> {
    // Always succed since we checked previously that it was mergeable
    Some(
        left.into_iter()
            .zip(right)
            .map(|pair| match pair {
                (InsSeqNode::Node(left), InsSeqNode::Node(right)) => InsSeqNode::Node(Subtree {
                    field: left.field,
                    node: merge_ins_nodes(left.node, right.node),
                }),
                (InsSeqNode::DeleteConflict(left), InsSeqNode::DeleteConflict(right))
                | (InsSeqNode::DeleteConflict(left), InsSeqNode::Node(right))
                | (InsSeqNode::Node(left), InsSeqNode::DeleteConflict(right)) => {
                    InsSeqNode::DeleteConflict(Subtree {
                        field: left.field,
                        node: merge_ins_nodes(left.node, right.node),
                    })
                }
                (InsSeqNode::InsertOrderConflict(_), _)
                | (_, InsSeqNode::InsertOrderConflict(_)) => {
                    panic!("Trying to merge insert tree sequences with order conflicts")
                }
            })
            .collect(),
    )
}

fn flatten_ins_spine(ins_spine: InsSpineNode) -> InsNode {
    match ins_spine {
        InsSpineNode::Spine(ins_subtree) => InsNode::InPlace(Colored::new_white(
            ins_subtree.convert_into(flatten_ins_spine_seq),
        )),
        InsSpineNode::Unchanged(mv) => InsNode::Elided(mv),
        InsSpineNode::Changed(ins) => ins,
    }
}

fn flatten_ins_spine_seq(ins_spine_seq: Vec<InsSpineSeqNode>) -> Vec<InsSeqNode> {
    let mut ins_seq = Vec::new();
    for ins_spine_seq_node in ins_spine_seq {
        match ins_spine_seq_node {
            InsSpineSeqNode::Zipped(ins_subtree) => {
                ins_seq.push(InsSeqNode::Node(ins_subtree.map(flatten_ins_spine)))
            }
            InsSpineSeqNode::Deleted => (),
            InsSpineSeqNode::DeleteConflict(confl) => {
                ins_seq.push(InsSeqNode::DeleteConflict(confl))
            }
            InsSpineSeqNode::Inserted(ins_list) => {
                for ins in ins_list.data {
                    ins_seq.push(InsSeqNode::Node(ins))
                }
            }
            InsSpineSeqNode::InsertOrderConflict(confl) => {
                ins_seq.push(InsSeqNode::InsertOrderConflict(confl))
            }
        }
    }
    ins_seq
}

fn can_inline_ins_in_del(ins: &InsNode, del: &DelNode) -> bool {
    match (del, ins) {
        (DelNode::Elided(_), _) => true,
        (DelNode::MetavariableConflict(_, _, _), _) => true,
        (DelNode::InPlace(del_subtree), InsNode::InPlace(ins_subtree)) => {
            if ins_subtree.colors == ColorSet::white() {
                Tree::compare(
                    &ins_subtree.data,
                    &del_subtree.data,
                    can_inline_ins_seq_in_del,
                )
            } else {
                // I don't really see how this branch can be triggered on real examples,
                // but refuse to merge for color preservation
                false
            }
        }
        _ => false,
    }
}

fn can_inline_ins_seq_in_del(ins_seq: &[InsSeqNode], del_seq: &[Subtree<DelNode>]) -> bool {
    if ins_seq.len() != del_seq.len() {
        return false;
    }

    ins_seq
        .iter()
        .zip(del_seq)
        .all(|(ins_seq_node, del)| match ins_seq_node {
            InsSeqNode::Node(ins) if del.field == ins.field => {
                can_inline_ins_in_del(&ins.node, &del.node)
            }
            _ => false,
        })
}

fn can_inline_ins_spine_in_del(ins: &InsSpineNode, del: &DelNode) -> bool {
    match ins {
        InsSpineNode::Spine(ins_subtree) => match del {
            DelNode::InPlace(del_subtree) => Tree::compare(
                ins_subtree,
                &del_subtree.data,
                can_inline_ins_spine_seq_in_del,
            ),
            DelNode::Elided(_) | DelNode::MetavariableConflict(_, _, _) => true,
        },
        InsSpineNode::Unchanged(_) => true,
        InsSpineNode::Changed(ins) => can_inline_ins_in_del(ins, del),
    }
}

fn can_inline_ins_spine_seq_in_del(
    ins_seq: &[InsSpineSeqNode],
    del_seq: &[Subtree<DelNode>],
) -> bool {
    if ins_seq.len() != del_seq.len() {
        return false;
    }

    ins_seq
        .iter()
        .zip(del_seq)
        .all(|(ins_spine_seq_node, del)| match ins_spine_seq_node {
            InsSpineSeqNode::Zipped(ins) if del.field == ins.field => {
                can_inline_ins_spine_in_del(&ins.node, &del.node)
            }
            InsSpineSeqNode::Deleted => true, // This could be false to disallow nested deletions
            _ => false,
        })
}

fn inline_ins_in_del<'t>(
    ins: InsNode<'t>,
    del: DelNode<'t>,
    metavars_status: &mut [MetavarInsReplacementList<'t>],
) -> DelNode<'t> {
    match del {
        DelNode::Elided(mv) => {
            // Here we may have to clone the insert tree once to check for potential conflicts
            metavars_status[mv.data.0].push(MetavarInsReplacement::Inlined(ins.clone()));
            DelNode::MetavariableConflict(
                mv.data,
                Box::new(DelNode::Elided(mv)),
                MetavarInsReplacement::Inlined(ins),
            )
        }
        DelNode::MetavariableConflict(mv, del, MetavarInsReplacement::InferFromDel) => {
            metavars_status[mv.0].push(MetavarInsReplacement::Inlined(ins.clone()));
            DelNode::MetavariableConflict(mv, del, MetavarInsReplacement::Inlined(ins))
        }
        DelNode::MetavariableConflict(mv, del, MetavarInsReplacement::Inlined(conflict_ins)) => {
            let merged_ins = merge_ins_nodes(conflict_ins, ins);
            metavars_status[mv.0].push(MetavarInsReplacement::Inlined(merged_ins.clone()));
            DelNode::MetavariableConflict(mv, del, MetavarInsReplacement::Inlined(merged_ins))
        }
        DelNode::InPlace(del_subtree) => match ins {
            InsNode::InPlace(ins_subtree) => DelNode::InPlace(Colored {
                data: Tree::merge_into(ins_subtree.data, del_subtree.data, |ins, del| {
                    inline_ins_seq_in_del(ins, del, metavars_status)
                })
                .unwrap(),
                colors: del_subtree.colors,
            }),
            _ => panic!("inline_ins_in_del() called with incompatible trees"),
        },
    }
}

fn inline_ins_seq_in_del<'t>(
    ins_seq: Vec<InsSeqNode<'t>>,
    del_seq: Vec<Subtree<DelNode<'t>>>,
    metavars_status: &mut [MetavarInsReplacementList<'t>],
) -> Option<Vec<Subtree<DelNode<'t>>>> {
    Some(
        ins_seq
            .into_iter()
            .zip(del_seq)
            .map(|(ins_seq_node, del)| match ins_seq_node {
                InsSeqNode::Node(ins) => {
                    del.map(|del| inline_ins_in_del(ins.node, del, metavars_status))
                }
                _ => panic!("InsSeq contains a conflict when merged with a deletion tree"),
            })
            .collect(),
    )
}

fn inline_ins_spine_in_del<'t>(
    ins_spine: InsSpineNode<'t>,
    mut del: DelNode<'t>,
    metavars_status: &mut [MetavarInsReplacementList<'t>],
) -> DelNode<'t> {
    match ins_spine {
        InsSpineNode::Spine(ins_subtree) => match del {
            DelNode::InPlace(del_subtree) => DelNode::InPlace(Colored {
                data: Tree::merge_into(ins_subtree, del_subtree.data, |ins, del| {
                    inline_ins_spine_seq_in_del(ins, del, metavars_status)
                })
                .unwrap(),
                colors: del_subtree.colors,
            }),
            DelNode::Elided(_) | DelNode::MetavariableConflict(_, _, _) => inline_ins_in_del(
                flatten_ins_spine(InsSpineNode::Spine(ins_subtree)),
                del,
                metavars_status,
            ),
        },
        InsSpineNode::Unchanged(_) => {
            register_kept_metavars(&mut del, metavars_status);
            del
        }
        InsSpineNode::Changed(ins) => inline_ins_in_del(ins, del, metavars_status),
    }
}

fn inline_ins_spine_seq_in_del<'t>(
    ins_spine_seq: Vec<InsSpineSeqNode<'t>>,
    del_seq: Vec<Subtree<DelNode<'t>>>,
    metavars_status: &mut [MetavarInsReplacementList<'t>],
) -> Option<Vec<Subtree<DelNode<'t>>>> {
    Some(
        ins_spine_seq
            .into_iter()
            .zip(del_seq)
            .map(|(ins_spine_seq_node, mut del)| match ins_spine_seq_node {
                InsSpineSeqNode::Zipped(ins_spine) => {
                    del.map(|del| inline_ins_spine_in_del(ins_spine.node, del, metavars_status))
                }
                InsSpineSeqNode::Deleted => {
                    register_kept_metavars(&mut del.node, metavars_status);
                    del
                }
                _ => panic!("InsSpineSeq cannot be inlined in the deletion tree"),
            })
            .collect(),
    )
}

fn register_kept_metavars<'t>(
    del: &mut DelNode<'t>,
    metavars_status: &mut [MetavarInsReplacementList<'t>],
) {
    match del {
        DelNode::InPlace(del_subtree) => del_subtree
            .data
            .visit_mut(|node| register_kept_metavars(&mut node.node, metavars_status)),
        DelNode::Elided(mv) => {
            metavars_status[mv.data.0].push(MetavarInsReplacement::InferFromDel);
            *del = DelNode::MetavariableConflict(
                mv.data,
                Box::new(DelNode::Elided(*mv)),
                MetavarInsReplacement::InferFromDel,
            )
        }
        DelNode::MetavariableConflict(mv, del, repl) => {
            metavars_status[mv.0].push(repl.clone());
            register_kept_metavars(del, metavars_status)
        }
    }
}

fn merge_ins_in_spine<'t>(
    node: AlignedSpineNode<'t>,
    metavars_status: &mut [MetavarInsReplacementList<'t>],
) -> InsMergedSpineNode<'t> {
    match node {
        AlignedSpineNode::Spine(spine) => InsMergedSpineNode::Spine(
            spine.map_children_into(|ch| merge_ins_in_spine_seq_node(ch, metavars_status)),
        ),
        AlignedSpineNode::Unchanged => InsMergedSpineNode::Unchanged,
        AlignedSpineNode::OneChange(mut del, ins) => {
            register_kept_metavars(&mut del, metavars_status);
            InsMergedSpineNode::OneChange(del, ins)
        }
        AlignedSpineNode::BothChanged(mut left_del, left_ins, mut right_del, right_ins) => {
            match (
                can_inline_ins_spine_in_del(&left_ins, &right_del),
                can_inline_ins_spine_in_del(&right_ins, &left_del),
            ) {
                (true, true) | (false, false) => {
                    register_kept_metavars(&mut left_del, metavars_status);
                    register_kept_metavars(&mut right_del, metavars_status);
                    InsMergedSpineNode::BothChanged(
                        left_del,
                        right_del,
                        merge_ins_nodes(flatten_ins_spine(left_ins), flatten_ins_spine(right_ins)),
                    )
                }
                (true, false) => {
                    let right_del = inline_ins_spine_in_del(left_ins, right_del, metavars_status);
                    register_kept_metavars(&mut left_del, metavars_status);
                    InsMergedSpineNode::BothChanged(
                        right_del,
                        left_del,
                        flatten_ins_spine(right_ins),
                    )
                }
                (false, true) => {
                    let left_del = inline_ins_spine_in_del(right_ins, left_del, metavars_status);
                    register_kept_metavars(&mut right_del, metavars_status);
                    InsMergedSpineNode::BothChanged(
                        left_del,
                        right_del,
                        flatten_ins_spine(left_ins),
                    )
                }
            }
        }
    }
}

fn merge_ins_in_spine_seq_node<'t>(
    seq_node: AlignedSpineSeqNode<'t>,
    metavars_status: &mut [MetavarInsReplacementList<'t>],
) -> InsMergedSpineSeqNode<'t> {
    match seq_node {
        AlignedSpineSeqNode::Zipped(node) => InsMergedSpineSeqNode::Zipped(
            node.map(|node| merge_ins_in_spine(node, metavars_status)),
        ),
        AlignedSpineSeqNode::BothDeleted(field, mut left_del, mut right_del) => {
            register_kept_metavars(&mut left_del, metavars_status);
            register_kept_metavars(&mut right_del, metavars_status);
            InsMergedSpineSeqNode::BothDeleted(field, left_del, right_del)
        }
        AlignedSpineSeqNode::OneDeleteConflict(field, mut del, mut conflict_del, conflict_ins) => {
            if can_inline_ins_spine_in_del(&conflict_ins, &del) {
                register_kept_metavars(&mut conflict_del, metavars_status);
                InsMergedSpineSeqNode::BothDeleted(
                    field,
                    inline_ins_spine_in_del(conflict_ins, del, metavars_status),
                    conflict_del,
                )
            } else {
                register_kept_metavars(&mut del, metavars_status);
                register_kept_metavars(&mut conflict_del, metavars_status);
                InsMergedSpineSeqNode::DeleteConflict(
                    field,
                    del,
                    conflict_del,
                    flatten_ins_spine(conflict_ins),
                )
            }
        }
        AlignedSpineSeqNode::BothDeleteConflict(
            field,
            mut left_del,
            left_ins,
            mut right_del,
            right_ins,
        ) => {
            match (
                can_inline_ins_spine_in_del(&left_ins, &right_del),
                can_inline_ins_spine_in_del(&right_ins, &left_del),
            ) {
                (false, false) => {
                    register_kept_metavars(&mut left_del, metavars_status);
                    register_kept_metavars(&mut right_del, metavars_status);
                    InsMergedSpineSeqNode::DeleteConflict(
                        field,
                        left_del,
                        right_del,
                        merge_ins_nodes(flatten_ins_spine(left_ins), flatten_ins_spine(right_ins)),
                    )
                }
                (true, false) => {
                    register_kept_metavars(&mut left_del, metavars_status);
                    InsMergedSpineSeqNode::DeleteConflict(
                        field,
                        inline_ins_spine_in_del(left_ins, right_del, metavars_status),
                        left_del,
                        flatten_ins_spine(right_ins),
                    )
                }
                (false, true) => {
                    register_kept_metavars(&mut right_del, metavars_status);
                    InsMergedSpineSeqNode::DeleteConflict(
                        field,
                        inline_ins_spine_in_del(right_ins, left_del, metavars_status),
                        right_del,
                        flatten_ins_spine(left_ins),
                    )
                }
                (true, true) => {
                    // Solve both conflicts at once!
                    InsMergedSpineSeqNode::BothDeleted(
                        field,
                        inline_ins_spine_in_del(right_ins, left_del, metavars_status),
                        inline_ins_spine_in_del(left_ins, right_del, metavars_status),
                    )
                }
            }
        }
        AlignedSpineSeqNode::Inserted(ins_vec) => InsMergedSpineSeqNode::Inserted(ins_vec),
    }
}

pub fn merge_ins(
    input: AlignedSpineNode,
    nb_vars: usize,
) -> (InsMergedSpineNode, Vec<MetavarInsReplacementList>) {
    let mut metavars_status = Vec::new();
    metavars_status.resize_with(nb_vars, Vec::new);
    let output = merge_ins_in_spine(input, &mut metavars_status);
    (output, metavars_status)
}
