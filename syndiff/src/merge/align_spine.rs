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

fn merge_spine_subtrees<'t>(
    left: Vec<ColoredSpineSeqNode<'t>>,
    right: Vec<ColoredSpineSeqNode<'t>>,
    next_metavar: &mut usize,
) -> Option<Vec<AlignedSpineSeqNode<'t>>> {
    let mut left_iter = flatten_del(left).peekable();
    let mut right_iter = flatten_del(right).peekable();
    let mut merged_subtrees = Vec::new();

    let into_ins_list = |node| match node {
        FlatDelSubtree::Inserted(ins_list) => ins_list,
        _ => unreachable!(),
    };

    while left_iter.peek().is_some() || right_iter.peek().is_some() {
        merged_subtrees.push(match (left_iter.peek(), right_iter.peek()) {
            (Some(FlatDelSubtree::Inserted(_)), Some(FlatDelSubtree::Inserted(_))) => {
                // Insertion in both sides, consume both
                let left_ins = into_ins_list(left_iter.next().unwrap());
                let right_ins = into_ins_list(right_iter.next().unwrap());
                AlignedSpineSeqNode::InsertOrderConflict(left_ins, right_ins)
            }
            (Some(FlatDelSubtree::Inserted(_)), _) => {
                // Only left side is an insertion, output it and continue.
                AlignedSpineSeqNode::Inserted(into_ins_list(left_iter.next().unwrap()))
            }
            (_, Some(FlatDelSubtree::Inserted(_))) => {
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
                    (FlatDelSubtree::Deleted(del), FlatDelSubtree::Zipped(spine))
                    | (FlatDelSubtree::Zipped(spine), FlatDelSubtree::Deleted(del)) => {
                        if del.field != spine.field {
                            return None;
                        }
                        let (spine_del, spine_ins) = split_spine(spine.node, next_metavar);
                        AlignedSpineSeqNode::DeleteConflict(
                            del.field, del.node, spine_del, spine_ins,
                        )
                    }
                    (FlatDelSubtree::Inserted(_), _) | (_, FlatDelSubtree::Inserted(_)) => {
                        unreachable!()
                    }
                }
            }
        })
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
