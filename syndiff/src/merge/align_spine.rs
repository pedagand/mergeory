use super::colors::{Colored, ColoredChangeNode, ColoredSpineNode, ColoredSpineSeqNode};
use crate::generic_tree::{FieldId, Subtree, Tree};
use crate::Metavariable;

type InsNode<'t> = ColoredChangeNode<'t>;
type DelNode<'t> = ColoredChangeNode<'t>;

pub enum InsSpineNode<'t> {
    Spine(Tree<'t, InsSpineSeqNode<'t>>),
    Unchanged(Metavariable),
    Changed(InsNode<'t>),
}

pub enum InsSpineSeqNode<'t> {
    Zipped(Subtree<InsSpineNode<'t>>),
    Deleted,
    Inserted(Vec<Subtree<InsNode<'t>>>),
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
    MovedAway(
        Option<FieldId>,
        Colored<Metavariable>,
        DelNode<'t>,
        Vec<InsNode<'t>>,
        InsSpineNode<'t>,
        Vec<InsNode<'t>>,
    ),
    DeleteConflict(Option<FieldId>, DelNode<'t>, DelNode<'t>, InsSpineNode<'t>),
    Inserted(Vec<Subtree<InsNode<'t>>>),
    InsertOrderConflict(Vec<Subtree<InsNode<'t>>>, Vec<Subtree<InsNode<'t>>>),
}

fn split_spine<'t>(
    tree: ColoredSpineNode<'t>,
    next_metavar: &mut usize,
) -> (DelNode<'t>, InsSpineNode<'t>) {
    match tree {
        ColoredSpineNode::Spine(spine) => {
            let (del, ins) =
                spine.split_into(|subtrees| split_spine_subtrees(subtrees, next_metavar));
            (
                DelNode::InPlace(Colored::new_white(del)),
                InsSpineNode::Spine(ins),
            )
        }
        ColoredSpineNode::Unchanged => {
            let new_metavar = Metavariable(*next_metavar);
            *next_metavar += 1;
            (
                DelNode::Elided(Colored::new_white(new_metavar)),
                InsSpineNode::Unchanged(new_metavar),
            )
        }
        ColoredSpineNode::Changed(del, ins) => (del, InsSpineNode::Changed(ins)),
    }
}

fn split_spine_subtrees<'t>(
    subtrees: Vec<ColoredSpineSeqNode<'t>>,
    next_metavar: &mut usize,
) -> (Vec<Subtree<DelNode<'t>>>, Vec<InsSpineSeqNode<'t>>) {
    let mut del_seq = Vec::new();
    let mut ins_seq = Vec::new();
    for subtree in subtrees {
        match subtree {
            ColoredSpineSeqNode::Zipped(node) => {
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
            ColoredSpineSeqNode::Deleted(del_list) => {
                for del in del_list {
                    del_seq.push(del);
                    ins_seq.push(InsSpineSeqNode::Deleted);
                }
            }
            ColoredSpineSeqNode::Inserted(ins_list) => {
                ins_seq.push(InsSpineSeqNode::Inserted(ins_list));
            }
        }
    }
    (del_seq, ins_seq)
}

fn merge_spines<'t>(
    left: ColoredSpineNode<'t>,
    right: ColoredSpineNode<'t>,
    next_metavar: &mut usize,
) -> Option<AlignedSpineNode<'t>> {
    Some(match (left, right) {
        (ColoredSpineNode::Spine(left_spine), ColoredSpineNode::Spine(right_spine)) => {
            AlignedSpineNode::Spine(Tree::merge_into(left_spine, right_spine, |l, r| {
                merge_spine_subtrees(l, r, next_metavar)
            })?)
        }
        (ColoredSpineNode::Unchanged, ColoredSpineNode::Unchanged) => AlignedSpineNode::Unchanged,
        (ColoredSpineNode::Spine(spine), ColoredSpineNode::Unchanged)
        | (ColoredSpineNode::Unchanged, ColoredSpineNode::Spine(spine)) => {
            align_spine_with_unchanged(ColoredSpineNode::Spine(spine), next_metavar)
        }
        (ColoredSpineNode::Changed(del, ins), ColoredSpineNode::Unchanged)
        | (ColoredSpineNode::Unchanged, ColoredSpineNode::Changed(del, ins)) => {
            AlignedSpineNode::OneChange(del, ins)
        }
        (
            ColoredSpineNode::Changed(left_del, left_ins),
            ColoredSpineNode::Changed(right_del, right_ins),
        ) => AlignedSpineNode::BothChanged(
            left_del,
            InsSpineNode::Changed(left_ins),
            right_del,
            InsSpineNode::Changed(right_ins),
        ),
        (ColoredSpineNode::Changed(del, ins), ColoredSpineNode::Spine(spine))
        | (ColoredSpineNode::Spine(spine), ColoredSpineNode::Changed(del, ins)) => {
            let (spine_del, spine_ins) = split_spine(ColoredSpineNode::Spine(spine), next_metavar);
            AlignedSpineNode::BothChanged(del, InsSpineNode::Changed(ins), spine_del, spine_ins)
        }
    })
}

enum DelAlignedSubtree<'t> {
    Zipped {
        ins_before: Vec<Subtree<InsNode<'t>>>,
        zipped_spine: Subtree<ColoredSpineNode<'t>>,
        ins_after: Vec<Subtree<InsNode<'t>>>,
    },
    Deleted(Subtree<DelNode<'t>>),
    InsertedAlone(Vec<Subtree<InsNode<'t>>>),
}

fn try_get_unified_field(seq: &[Subtree<InsNode>]) -> Option<Option<FieldId>> {
    let mut without_sep_iter = seq.iter().filter(|subtree| {
        !matches!(
            subtree,
            Subtree {
                field: None,
                node: InsNode::InPlace(Colored {
                    data: Tree::Leaf(_),
                    ..
                })
            }
        )
    });
    let first_subtree = without_sep_iter.next()?;
    for subtree in without_sep_iter {
        if subtree.field != first_subtree.field {
            return None;
        }
    }
    Some(first_subtree.field)
}

fn is_spine_separator(subtree: &Subtree<ColoredSpineNode>) -> bool {
    matches!(
        subtree,
        Subtree {
            field: None,
            node: ColoredSpineNode::Spine(Tree::Leaf(_))
        }
    )
}

fn align_on_del(seq: Vec<ColoredSpineSeqNode>) -> Vec<DelAlignedSubtree> {
    let mut seq_iter = seq.into_iter().peekable();
    let mut del_aligned_seq = Vec::new();

    while let Some(node) = seq_iter.next() {
        match node {
            ColoredSpineSeqNode::Zipped(spine) => {
                let ins_before = match del_aligned_seq.last() {
                    Some(DelAlignedSubtree::InsertedAlone(ins_list))
                        if !is_spine_separator(&spine)
                            && try_get_unified_field(ins_list) == Some(spine.field) =>
                    {
                        match del_aligned_seq.pop() {
                            Some(DelAlignedSubtree::InsertedAlone(ins_list)) => ins_list,
                            _ => unreachable!(),
                        }
                    }
                    _ => Vec::new(),
                };
                del_aligned_seq.push(DelAlignedSubtree::Zipped {
                    ins_before,
                    zipped_spine: spine,
                    ins_after: Vec::new(),
                })
            }
            ColoredSpineSeqNode::Deleted(del_seq) => {
                for del in del_seq {
                    del_aligned_seq.push(DelAlignedSubtree::Deleted(del));
                }
            }
            ColoredSpineSeqNode::Inserted(ins_list) => match del_aligned_seq.last_mut() {
                Some(DelAlignedSubtree::Zipped {
                    zipped_spine,
                    ins_after,
                    ..
                }) if !is_spine_separator(zipped_spine)
                    && try_get_unified_field(&ins_list) == Some(zipped_spine.field) =>
                {
                    ins_after.extend(ins_list)
                }
                _ => del_aligned_seq.push(DelAlignedSubtree::InsertedAlone(ins_list)),
            },
        }
    }

    del_aligned_seq
}

fn merge_spine_subtrees<'t>(
    left: Vec<ColoredSpineSeqNode<'t>>,
    right: Vec<ColoredSpineSeqNode<'t>>,
    next_metavar: &mut usize,
) -> Option<Vec<AlignedSpineSeqNode<'t>>> {
    let mut left_iter = align_on_del(left).into_iter().peekable();
    let mut right_iter = align_on_del(right).into_iter().peekable();
    let mut merged_subtrees = Vec::new();

    while left_iter.peek().is_some() || right_iter.peek().is_some() {
        let into_ins_list = |node| match node {
            DelAlignedSubtree::InsertedAlone(ins_list) => ins_list,
            _ => unreachable!(),
        };

        match (left_iter.peek(), right_iter.peek()) {
            (
                Some(DelAlignedSubtree::InsertedAlone(_)),
                Some(DelAlignedSubtree::InsertedAlone(_)),
            ) => {
                // Insertion in both sides, consume both
                let left_ins = into_ins_list(left_iter.next().unwrap());
                let right_ins = into_ins_list(right_iter.next().unwrap());
                merged_subtrees.push(AlignedSpineSeqNode::InsertOrderConflict(
                    left_ins, right_ins,
                ))
            }
            (Some(DelAlignedSubtree::InsertedAlone(_)), _) => {
                // Only left side is an insertion, output it and continue.
                merged_subtrees.push(AlignedSpineSeqNode::Inserted(into_ins_list(
                    left_iter.next().unwrap(),
                )))
            }
            (_, Some(DelAlignedSubtree::InsertedAlone(_))) => {
                // Only right side is an insertion, output it and continue.
                merged_subtrees.push(AlignedSpineSeqNode::Inserted(into_ins_list(
                    right_iter.next().unwrap(),
                )))
            }
            _ => {
                // No insertion either in left or right, consume both or return None if not
                // possible
                match (left_iter.next()?, right_iter.next()?) {
                    (
                        DelAlignedSubtree::Zipped {
                            ins_before: left_ins_before,
                            zipped_spine: left_spine,
                            ins_after: left_ins_after,
                        },
                        DelAlignedSubtree::Zipped {
                            ins_before: right_ins_before,
                            zipped_spine: right_spine,
                            ins_after: right_ins_after,
                        },
                    ) => {
                        if left_spine.field != right_spine.field {
                            return None;
                        }

                        let push_merged_ins_seq =
                            |left_seq: Vec<_>, right_seq: Vec<_>, merged_subtrees: &mut Vec<_>| {
                                match (left_seq.is_empty(), right_seq.is_empty()) {
                                    (true, true) => (),
                                    (false, true) => merged_subtrees
                                        .push(AlignedSpineSeqNode::Inserted(left_seq)),
                                    (true, false) => merged_subtrees
                                        .push(AlignedSpineSeqNode::Inserted(right_seq)),
                                    (false, false) => merged_subtrees.push(
                                        AlignedSpineSeqNode::InsertOrderConflict(
                                            left_seq, right_seq,
                                        ),
                                    ),
                                }
                            };

                        push_merged_ins_seq(
                            left_ins_before,
                            right_ins_before,
                            &mut merged_subtrees,
                        );
                        merged_subtrees.push(AlignedSpineSeqNode::Zipped(Subtree {
                            field: left_spine.field,
                            node: merge_spines(left_spine.node, right_spine.node, next_metavar)?,
                        }));
                        push_merged_ins_seq(left_ins_after, right_ins_after, &mut merged_subtrees);
                    }
                    (
                        DelAlignedSubtree::Deleted(left_del),
                        DelAlignedSubtree::Deleted(right_del),
                    ) => {
                        if left_del.field != right_del.field {
                            return None;
                        }
                        merged_subtrees.push(AlignedSpineSeqNode::BothDeleted(
                            left_del.field,
                            left_del.node,
                            right_del.node,
                        ))
                    }
                    (
                        DelAlignedSubtree::Deleted(Subtree {
                            field: del_field,
                            node: DelNode::Elided(mv),
                        }),
                        DelAlignedSubtree::Zipped {
                            ins_before,
                            zipped_spine,
                            ins_after,
                        },
                    )
                    | (
                        DelAlignedSubtree::Zipped {
                            ins_before,
                            zipped_spine,
                            ins_after,
                        },
                        DelAlignedSubtree::Deleted(Subtree {
                            field: del_field,
                            node: DelNode::Elided(mv),
                        }),
                    ) => {
                        if del_field != zipped_spine.field {
                            return None;
                        }
                        let (spine_del, spine_ins) = split_spine(zipped_spine.node, next_metavar);
                        let ins_before =
                            ins_before.into_iter().map(|subtree| subtree.node).collect();
                        let ins_after = ins_after.into_iter().map(|subtree| subtree.node).collect();
                        merged_subtrees.push(AlignedSpineSeqNode::MovedAway(
                            del_field, mv, spine_del, ins_before, spine_ins, ins_after,
                        ))
                    }
                    (
                        DelAlignedSubtree::Deleted(del),
                        DelAlignedSubtree::Zipped {
                            ins_before,
                            zipped_spine,
                            ins_after,
                        },
                    )
                    | (
                        DelAlignedSubtree::Zipped {
                            ins_before,
                            zipped_spine,
                            ins_after,
                        },
                        DelAlignedSubtree::Deleted(del),
                    ) => {
                        if del.field != zipped_spine.field {
                            return None;
                        }
                        let (spine_del, spine_ins) = split_spine(zipped_spine.node, next_metavar);

                        if !ins_before.is_empty() {
                            merged_subtrees.push(AlignedSpineSeqNode::Inserted(ins_before));
                        }
                        merged_subtrees.push(AlignedSpineSeqNode::DeleteConflict(
                            del.field, del.node, spine_del, spine_ins,
                        ));
                        if !ins_after.is_empty() {
                            merged_subtrees.push(AlignedSpineSeqNode::Inserted(ins_after));
                        }
                    }
                    (DelAlignedSubtree::InsertedAlone(_), _)
                    | (_, DelAlignedSubtree::InsertedAlone(_)) => {
                        unreachable!()
                    }
                }
            }
        }
    }
    Some(merged_subtrees)
}

fn align_spine_with_unchanged<'t>(
    tree: ColoredSpineNode<'t>,
    next_metavar: &mut usize,
) -> AlignedSpineNode<'t> {
    match tree {
        ColoredSpineNode::Spine(spine) => AlignedSpineNode::Spine(
            spine.convert_into(|node| align_spine_subtrees_with_unchanged(node, next_metavar)),
        ),
        ColoredSpineNode::Unchanged => AlignedSpineNode::Unchanged,
        ColoredSpineNode::Changed(del, ins) => AlignedSpineNode::OneChange(del, ins),
    }
}

enum FlatDelSubtree<'t> {
    Zipped(Subtree<ColoredSpineNode<'t>>),
    Deleted(Subtree<DelNode<'t>>),
    Inserted(Vec<Subtree<InsNode<'t>>>),
}

fn flatten_del(seq: Vec<ColoredSpineSeqNode>) -> impl Iterator<Item = FlatDelSubtree> {
    seq.into_iter()
        .map::<Box<dyn Iterator<Item = FlatDelSubtree>>, _>(|subtree| match subtree {
            ColoredSpineSeqNode::Zipped(spine) => {
                Box::new(std::iter::once(FlatDelSubtree::Zipped(spine)))
            }
            ColoredSpineSeqNode::Deleted(del_seq) => {
                Box::new(del_seq.into_iter().map(FlatDelSubtree::Deleted))
            }
            ColoredSpineSeqNode::Inserted(ins_list) => {
                Box::new(std::iter::once(FlatDelSubtree::Inserted(ins_list)))
            }
        })
        .flatten()
}

fn align_spine_subtrees_with_unchanged<'t>(
    subtrees: Vec<ColoredSpineSeqNode<'t>>,
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
                AlignedSpineSeqNode::DeleteConflict(
                    del.field,
                    del.node,
                    unchanged_del,
                    unchanged_ins,
                )
            }
            FlatDelSubtree::Inserted(ins_seq) => AlignedSpineSeqNode::Inserted(ins_seq),
        })
        .collect()
}

pub fn align_spines<'t>(
    left: ColoredSpineNode<'t>,
    right: ColoredSpineNode<'t>,
    mut next_metavar: usize,
) -> Option<(AlignedSpineNode<'t>, usize)> {
    let merged = merge_spines(left, right, &mut next_metavar)?;
    Some((merged, next_metavar))
}
