use super::{Colored, DelNode, InsNode, SpineNode, SpineSeqNode};
use crate::generic_tree::{FieldId, Subtree, Tree};
use crate::Metavariable;

pub enum InsSpineNode<'t> {
    Spine(Tree<'t, InsSpineSeqNode<'t>>),
    Unchanged(Metavariable),
    Changed(InsNode<'t>),
}

pub enum InsSpineSeqNode<'t> {
    Zipped(Subtree<InsSpineNode<'t>>),
    Deleted,
    DeleteConflict(Subtree<InsNode<'t>>),
    Inserted(Colored<Vec<Subtree<InsNode<'t>>>>),
    InsertOrderConflict(Vec<Colored<Vec<Subtree<InsNode<'t>>>>>),
}

pub enum AlignedSpineNode<'t> {
    Spine(Tree<'t, AlignedSpineSeqNode<'t>>),
    Unchanged,
    OneChange(DelNode<'t>, InsNode<'t>),
    BothChanged(DelNode<'t>, InsSpineNode<'t>, DelNode<'t>, InsSpineNode<'t>),
}

pub enum AlignedSpineSeqNode<'t> {
    Zipped(Subtree<AlignedSpineNode<'t>>),
    BothDeleted(Option<FieldId>, DelNode<'t>, DelNode<'t>),
    OneDeleteConflict(Option<FieldId>, DelNode<'t>, DelNode<'t>, InsSpineNode<'t>),
    BothDeleteConflict(
        Option<FieldId>,
        DelNode<'t>,
        InsSpineNode<'t>,
        DelNode<'t>,
        InsSpineNode<'t>,
    ),
    Inserted(Vec<Colored<Vec<Subtree<InsNode<'t>>>>>),
}

fn split_spine<'t>(
    tree: SpineNode<'t>,
    next_metavar: &mut usize,
) -> (DelNode<'t>, InsSpineNode<'t>) {
    match tree {
        SpineNode::Spine(spine) => {
            let (del, ins) =
                spine.split_into(|subtrees| split_spine_subtrees(subtrees, next_metavar));
            (
                DelNode::InPlace(Colored::new_white(del)),
                InsSpineNode::Spine(ins),
            )
        }
        SpineNode::Unchanged => {
            let new_metavar = Metavariable(*next_metavar);
            *next_metavar += 1;
            (
                DelNode::Elided(Colored::new_white(new_metavar)),
                InsSpineNode::Unchanged(new_metavar),
            )
        }
        SpineNode::Changed(del, ins) => (del, InsSpineNode::Changed(ins)),
    }
}

fn split_spine_subtrees<'t>(
    subtrees: Vec<SpineSeqNode<'t>>,
    next_metavar: &mut usize,
) -> (Vec<Subtree<DelNode<'t>>>, Vec<InsSpineSeqNode<'t>>) {
    let mut del_seq = Vec::new();
    let mut ins_seq = Vec::new();
    for subtree in subtrees {
        match subtree {
            SpineSeqNode::Zipped(node) => {
                let (del, ins) = split_spine(node.node, next_metavar);
                del_seq.push(Subtree {
                    field: node.field,
                    node: del,
                });
                ins_seq.push(InsSpineSeqNode::Zipped(Subtree {
                    field: node.field,
                    node: ins,
                }));
            }
            SpineSeqNode::Deleted(del_list) => {
                for del in del_list {
                    del_seq.push(del);
                    ins_seq.push(InsSpineSeqNode::Deleted);
                }
            }
            SpineSeqNode::DeleteConflict(field, del, ins) => {
                del_seq.push(Subtree { field, node: del });
                ins_seq.push(InsSpineSeqNode::DeleteConflict(Subtree {
                    field,
                    node: ins,
                }));
            }
            SpineSeqNode::Inserted(ins_list) => {
                ins_seq.push(InsSpineSeqNode::Inserted(ins_list));
            }
            SpineSeqNode::InsertOrderConflict(ins_conflict) => {
                ins_seq.push(InsSpineSeqNode::InsertOrderConflict(ins_conflict));
            }
        }
    }
    (del_seq, ins_seq)
}

fn merge_spines<'t>(
    left: SpineNode<'t>,
    right: SpineNode<'t>,
    next_metavar: &mut usize,
) -> Option<AlignedSpineNode<'t>> {
    Some(match (left, right) {
        (SpineNode::Spine(left_spine), SpineNode::Spine(right_spine)) => {
            AlignedSpineNode::Spine(Tree::merge_into(left_spine, right_spine, |l, r| {
                merge_spine_subtrees(l, r, next_metavar)
            })?)
        }
        (SpineNode::Unchanged, SpineNode::Unchanged) => AlignedSpineNode::Unchanged,
        (SpineNode::Spine(spine), SpineNode::Unchanged)
        | (SpineNode::Unchanged, SpineNode::Spine(spine)) => {
            align_spine_with_unchanged(SpineNode::Spine(spine), next_metavar)
        }
        (SpineNode::Changed(del, ins), SpineNode::Unchanged)
        | (SpineNode::Unchanged, SpineNode::Changed(del, ins)) => {
            AlignedSpineNode::OneChange(del, ins)
        }
        (SpineNode::Changed(left_del, left_ins), SpineNode::Changed(right_del, right_ins)) => {
            AlignedSpineNode::BothChanged(
                left_del,
                InsSpineNode::Changed(left_ins),
                right_del,
                InsSpineNode::Changed(right_ins),
            )
        }
        (SpineNode::Changed(del, ins), SpineNode::Spine(spine))
        | (SpineNode::Spine(spine), SpineNode::Changed(del, ins)) => {
            let (spine_del, spine_ins) = split_spine(SpineNode::Spine(spine), next_metavar);
            AlignedSpineNode::BothChanged(del, InsSpineNode::Changed(ins), spine_del, spine_ins)
        }
    })
}

enum FlatDelSubtree<'t> {
    Zipped(Subtree<SpineNode<'t>>),
    Deleted(Subtree<DelNode<'t>>),
    DeleteConflict(Option<FieldId>, DelNode<'t>, InsNode<'t>),
    Inserted(Colored<Vec<Subtree<InsNode<'t>>>>),
    InsertOrderConflict(Vec<Colored<Vec<Subtree<InsNode<'t>>>>>),
}

fn flatten_del(seq: Vec<SpineSeqNode>) -> impl Iterator<Item = FlatDelSubtree> {
    seq.into_iter()
        .map::<Box<dyn Iterator<Item = FlatDelSubtree>>, _>(|subtree| match subtree {
            SpineSeqNode::Zipped(spine) => Box::new(std::iter::once(FlatDelSubtree::Zipped(spine))),
            SpineSeqNode::Deleted(del_seq) => {
                Box::new(del_seq.into_iter().map(FlatDelSubtree::Deleted))
            }
            SpineSeqNode::DeleteConflict(f, del, ins) => {
                Box::new(std::iter::once(FlatDelSubtree::DeleteConflict(f, del, ins)))
            }
            SpineSeqNode::Inserted(ins_list) => {
                Box::new(std::iter::once(FlatDelSubtree::Inserted(ins_list)))
            }
            SpineSeqNode::InsertOrderConflict(conflicts) => Box::new(std::iter::once(
                FlatDelSubtree::InsertOrderConflict(conflicts),
            )),
        })
        .flatten()
}

fn merge_spine_subtrees<'t>(
    left: Vec<SpineSeqNode<'t>>,
    right: Vec<SpineSeqNode<'t>>,
    next_metavar: &mut usize,
) -> Option<Vec<AlignedSpineSeqNode<'t>>> {
    let mut left_iter = flatten_del(left).peekable();
    let mut right_iter = flatten_del(right).peekable();
    let mut merged_subtrees = Vec::new();

    let into_ins_list = |node| match node {
        FlatDelSubtree::Inserted(ins) => vec![ins],
        FlatDelSubtree::InsertOrderConflict(list) => list,
        _ => unreachable!(),
    };

    while left_iter.peek().is_some() || right_iter.peek().is_some() {
        merged_subtrees.push(match (left_iter.peek(), right_iter.peek()) {
            (
                Some(&FlatDelSubtree::Inserted(_) | &FlatDelSubtree::InsertOrderConflict(_)),
                Some(&FlatDelSubtree::Inserted(_) | &FlatDelSubtree::InsertOrderConflict(_)),
            ) => {
                // Insertion in both sides, consume both
                let mut ins_list = into_ins_list(left_iter.next().unwrap());
                ins_list.extend(into_ins_list(right_iter.next().unwrap()));
                AlignedSpineSeqNode::Inserted(ins_list)
            }
            (Some(&FlatDelSubtree::Inserted(_) | &FlatDelSubtree::InsertOrderConflict(_)), _) => {
                // Only left side is an insertion, output it and continue.
                AlignedSpineSeqNode::Inserted(into_ins_list(left_iter.next().unwrap()))
            }
            (_, Some(&FlatDelSubtree::Inserted(_) | &FlatDelSubtree::InsertOrderConflict(_))) => {
                // Only right side is an insertion, output it and continue.
                AlignedSpineSeqNode::Inserted(into_ins_list(right_iter.next().unwrap()))
            }
            _ => {
                // No insertion either in left or right, consume both or return None if not
                // possible
                match (left_iter.next()?, right_iter.next()?) {
                    (FlatDelSubtree::Zipped(left_spine), FlatDelSubtree::Zipped(right_spine)) => {
                        if left_spine.field != right_spine.field {
                            return None;
                        }
                        AlignedSpineSeqNode::Zipped(Subtree {
                            field: left_spine.field,
                            node: merge_spines(left_spine.node, right_spine.node, next_metavar)?,
                        })
                    }
                    (FlatDelSubtree::Deleted(left_del), FlatDelSubtree::Deleted(right_del)) => {
                        if left_del.field != right_del.field {
                            return None;
                        }
                        AlignedSpineSeqNode::BothDeleted(
                            left_del.field,
                            left_del.node,
                            right_del.node,
                        )
                    }
                    (
                        FlatDelSubtree::DeleteConflict(field, c_del, c_ins),
                        FlatDelSubtree::Deleted(del),
                    )
                    | (
                        FlatDelSubtree::Deleted(del),
                        FlatDelSubtree::DeleteConflict(field, c_del, c_ins),
                    ) => {
                        if field != del.field {
                            return None;
                        }
                        AlignedSpineSeqNode::OneDeleteConflict(
                            field,
                            del.node,
                            c_del,
                            InsSpineNode::Changed(c_ins),
                        )
                    }
                    (
                        FlatDelSubtree::DeleteConflict(left_field, left_del, left_ins),
                        FlatDelSubtree::DeleteConflict(right_field, right_del, right_ins),
                    ) => {
                        if left_field != right_field {
                            return None;
                        }
                        AlignedSpineSeqNode::BothDeleteConflict(
                            left_field,
                            left_del,
                            InsSpineNode::Changed(left_ins),
                            right_del,
                            InsSpineNode::Changed(right_ins),
                        )
                    }
                    (FlatDelSubtree::Deleted(del), FlatDelSubtree::Zipped(spine))
                    | (FlatDelSubtree::Zipped(spine), FlatDelSubtree::Deleted(del)) => {
                        if del.field != spine.field {
                            return None;
                        }
                        let (spine_del, spine_ins) = split_spine(spine.node, next_metavar);
                        AlignedSpineSeqNode::OneDeleteConflict(
                            del.field, del.node, spine_del, spine_ins,
                        )
                    }
                    (
                        FlatDelSubtree::DeleteConflict(field, c_del, c_ins),
                        FlatDelSubtree::Zipped(spine),
                    )
                    | (
                        FlatDelSubtree::Zipped(spine),
                        FlatDelSubtree::DeleteConflict(field, c_del, c_ins),
                    ) => {
                        if field != spine.field {
                            return None;
                        }
                        let (spine_del, spine_ins) = split_spine(spine.node, next_metavar);
                        AlignedSpineSeqNode::BothDeleteConflict(
                            field,
                            spine_del,
                            spine_ins,
                            c_del,
                            InsSpineNode::Changed(c_ins),
                        )
                    }
                    (FlatDelSubtree::Inserted(_), _) | (_, FlatDelSubtree::Inserted(_)) => {
                        unreachable!()
                    }
                    (FlatDelSubtree::InsertOrderConflict(_), _)
                    | (_, FlatDelSubtree::InsertOrderConflict(_)) => unreachable!(),
                }
            }
        })
    }
    Some(merged_subtrees)
}

fn align_spine_with_unchanged<'t>(
    tree: SpineNode<'t>,
    next_metavar: &mut usize,
) -> AlignedSpineNode<'t> {
    match tree {
        SpineNode::Spine(spine) => AlignedSpineNode::Spine(
            spine.convert_into(|node| align_spine_subtrees_with_unchanged(node, next_metavar)),
        ),
        SpineNode::Unchanged => AlignedSpineNode::Unchanged,
        SpineNode::Changed(del, ins) => AlignedSpineNode::OneChange(del, ins),
    }
}

fn align_spine_subtrees_with_unchanged<'t>(
    subtrees: Vec<SpineSeqNode<'t>>,
    next_metavar: &mut usize,
) -> Vec<AlignedSpineSeqNode<'t>> {
    flatten_del(subtrees)
        .map(|subtree| match subtree {
            FlatDelSubtree::Zipped(node) => AlignedSpineSeqNode::Zipped(
                node.map(|node| align_spine_with_unchanged(node, next_metavar)),
            ),
            FlatDelSubtree::Deleted(del) => {
                let unchanged_mv = Metavariable(*next_metavar);
                *next_metavar += 1;
                let unchanged_del = DelNode::Elided(Colored::new_white(unchanged_mv));
                let unchanged_ins = InsSpineNode::Unchanged(unchanged_mv);
                AlignedSpineSeqNode::OneDeleteConflict(
                    del.field,
                    del.node,
                    unchanged_del,
                    unchanged_ins,
                )
            }
            FlatDelSubtree::DeleteConflict(field, conflict_del, conflict_ins) => {
                let unchanged_mv = Metavariable(*next_metavar);
                *next_metavar += 1;
                let unchanged_del = DelNode::Elided(Colored::new_white(unchanged_mv));
                let unchanged_ins = InsSpineNode::Unchanged(unchanged_mv);
                AlignedSpineSeqNode::BothDeleteConflict(
                    field,
                    conflict_del,
                    InsSpineNode::Changed(conflict_ins),
                    unchanged_del,
                    unchanged_ins,
                )
            }
            FlatDelSubtree::Inserted(ins_seq) => AlignedSpineSeqNode::Inserted(vec![ins_seq]),
            FlatDelSubtree::InsertOrderConflict(ins_conflict) => {
                AlignedSpineSeqNode::Inserted(ins_conflict)
            }
        })
        .collect()
}

pub fn align_spines<'t>(
    left: SpineNode<'t>,
    right: SpineNode<'t>,
    mut next_metavar: usize,
) -> Option<(AlignedSpineNode<'t>, usize)> {
    let merged = merge_spines(left, right, &mut next_metavar)?;
    Some((merged, next_metavar))
}
