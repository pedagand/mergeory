use super::{Colored, DelNode, InsNode, InsSeq, InsSeqNode, SpineNode, SpineSeq, SpineSeqNode};
use crate::diff_tree::Metavariable;
use crate::family_traits::{Convert, Merge, Split};

pub enum MergeSpineNode<S, D, I> {
    Spine(S),
    Unchanged,
    OneChange(DelNode<D, I>, InsNode<I>),
    BothChanged(DelNode<D, I>, InsNode<I>, DelNode<D, I>, InsNode<I>),
}
pub enum MergeSpineSeqNode<S, D, I> {
    Zipped(MergeSpineNode<S, D, I>),
    BothDeleted(Colored<DelNode<D, I>>, Colored<DelNode<D, I>>),
    OneDeleteConflict(Colored<DelNode<D, I>>, Colored<DelNode<D, I>>, InsNode<I>),
    BothDeleteConflict(
        Colored<DelNode<D, I>>,
        InsNode<I>,
        Colored<DelNode<D, I>>,
        InsNode<I>,
    ),
    Insert(Vec<Colored<Vec<InsNode<I>>>>),
}
pub struct MergeSpineSeq<S, D, I>(pub Vec<MergeSpineSeqNode<S, D, I>>);

pub struct SpineAligner {
    next_metavar: usize,
}

impl<S, D, I> Split<SpineNode<S, D, I>, DelNode<D, I>, InsNode<I>> for SpineAligner
where
    SpineAligner: Split<S, D, I>,
{
    fn split(&mut self, input: SpineNode<S, D, I>) -> (DelNode<D, I>, InsNode<I>) {
        match input {
            SpineNode::Spine(s) => {
                let (del, ins) = self.split(s);
                (
                    DelNode::InPlace(del),
                    InsNode::InPlace(Colored::new_white(ins)),
                )
            }
            SpineNode::Unchanged => {
                let new_metavar = Metavariable(self.next_metavar);
                self.next_metavar += 1;
                (
                    DelNode::Ellided(new_metavar),
                    InsNode::Ellided(Colored::new_white(new_metavar)),
                )
            }
            SpineNode::Changed(del, ins) => (del, ins),
        }
    }
}

impl<S, D, I> Split<SpineSeq<S, D, I>, Vec<DelNode<D, I>>, InsSeq<I>> for SpineAligner
where
    SpineAligner: Split<SpineNode<S, D, I>, DelNode<D, I>, InsNode<I>>,
{
    fn split(&mut self, input: SpineSeq<S, D, I>) -> (Vec<DelNode<D, I>>, InsSeq<I>) {
        let mut del_seq = Vec::new();
        let mut ins_seq = Vec::new();
        for seq_node in input.0 {
            // FIXME: We lose a color here, this is weird, and probably unsound
            match seq_node {
                SpineSeqNode::Zipped(node) => {
                    let (del, ins) = self.split(node);
                    del_seq.push(del);
                    ins_seq.push(InsSeqNode::Node(ins));
                }
                SpineSeqNode::Deleted(del) => {
                    del_seq.push(del.node);
                }
                SpineSeqNode::DeleteConflict(del, ins) => {
                    del_seq.push(del.node);
                    ins_seq.push(InsSeqNode::DeleteConflict(ins));
                }
                SpineSeqNode::Inserted(ins_list) => {
                    for ins in ins_list.node {
                        ins_seq.push(InsSeqNode::Node(ins));
                    }
                }
                SpineSeqNode::InsertOrderConflict(ins_conflict) => {
                    ins_seq.push(InsSeqNode::InsertOrderConflict(ins_conflict));
                }
            }
        }
        (del_seq, InsSeq(ins_seq))
    }
}

impl<S, MS, D, I> Merge<SpineNode<S, D, I>, SpineNode<S, D, I>, MergeSpineNode<MS, D, I>>
    for SpineAligner
where
    SpineAligner: Merge<S, S, MS>,
    SpineAligner: Split<S, D, I>,
    SpineAligner: Convert<S, MS>,
{
    fn can_merge(&mut self, left: &SpineNode<S, D, I>, right: &SpineNode<S, D, I>) -> bool {
        // In SpineAligner we only check matching spines. More checks will be performed later.
        match (left, right) {
            (SpineNode::Spine(left), SpineNode::Spine(right)) => self.can_merge(left, right),
            _ => true,
        }
    }

    fn merge(
        &mut self,
        left: SpineNode<S, D, I>,
        right: SpineNode<S, D, I>,
    ) -> MergeSpineNode<MS, D, I> {
        match (left, right) {
            (SpineNode::Spine(left_spine), SpineNode::Spine(right_spine)) => {
                MergeSpineNode::Spine(self.merge(left_spine, right_spine))
            }
            (SpineNode::Unchanged, SpineNode::Unchanged) => MergeSpineNode::Unchanged,
            (SpineNode::Spine(spine), SpineNode::Unchanged)
            | (SpineNode::Unchanged, SpineNode::Spine(spine)) => {
                MergeSpineNode::Spine(self.convert(spine))
            }
            (SpineNode::Changed(del, ins), SpineNode::Unchanged)
            | (SpineNode::Unchanged, SpineNode::Changed(del, ins)) => {
                MergeSpineNode::OneChange(del, ins)
            }
            (SpineNode::Changed(left_del, left_ins), SpineNode::Changed(right_del, right_ins)) => {
                MergeSpineNode::BothChanged(left_del, left_ins, right_del, right_ins)
            }
            (SpineNode::Changed(del, ins), SpineNode::Spine(spine))
            | (SpineNode::Spine(spine), SpineNode::Changed(del, ins)) => {
                let (spine_del, spine_ins) = self.split(spine);
                let spine_del = DelNode::InPlace(spine_del);
                let spine_ins = InsNode::InPlace(Colored::new_white(spine_ins));
                MergeSpineNode::BothChanged(del, ins, spine_del, spine_ins)
            }
        }
    }
}

impl<S, MS, D, I> Merge<SpineSeqNode<S, D, I>, SpineSeqNode<S, D, I>, MergeSpineSeqNode<MS, D, I>>
    for SpineAligner
where
    SpineAligner: Merge<SpineNode<S, D, I>, SpineNode<S, D, I>, MergeSpineNode<MS, D, I>>,
    SpineAligner: Split<SpineNode<S, D, I>, DelNode<D, I>, InsNode<I>>,
{
    fn can_merge(&mut self, left: &SpineSeqNode<S, D, I>, right: &SpineSeqNode<S, D, I>) -> bool {
        match (left, right) {
            (SpineSeqNode::Zipped(left_spine), SpineSeqNode::Zipped(right_spine)) => {
                // Zipped subtrees should be mergeable
                self.can_merge(left_spine, right_spine)
            }
            // Insertions are handled at the SpineSeq level, do not merge them here
            (SpineSeqNode::Inserted(_), _) | (_, SpineSeqNode::Inserted(_)) => false,
            (SpineSeqNode::InsertOrderConflict(_), _)
            | (_, SpineSeqNode::InsertOrderConflict(_)) => false,
            // The rest is always mergeable
            _ => true,
        }
    }

    fn merge(
        &mut self,
        left: SpineSeqNode<S, D, I>,
        right: SpineSeqNode<S, D, I>,
    ) -> MergeSpineSeqNode<MS, D, I> {
        match (left, right) {
            (SpineSeqNode::Zipped(left_spine), SpineSeqNode::Zipped(right_spine)) => {
                MergeSpineSeqNode::Zipped(self.merge(left_spine, right_spine))
            }
            (SpineSeqNode::Deleted(left_del), SpineSeqNode::Deleted(right_del)) => {
                MergeSpineSeqNode::BothDeleted(left_del, right_del)
            }
            (SpineSeqNode::DeleteConflict(c_del, c_ins), SpineSeqNode::Deleted(del))
            | (SpineSeqNode::Deleted(del), SpineSeqNode::DeleteConflict(c_del, c_ins)) => {
                MergeSpineSeqNode::OneDeleteConflict(del, c_del, c_ins)
            }
            (
                SpineSeqNode::DeleteConflict(left_del, left_ins),
                SpineSeqNode::DeleteConflict(right_del, right_ins),
            ) => MergeSpineSeqNode::BothDeleteConflict(left_del, left_ins, right_del, right_ins),
            (SpineSeqNode::Deleted(del), SpineSeqNode::Zipped(spine))
            | (SpineSeqNode::Zipped(spine), SpineSeqNode::Deleted(del)) => {
                let (spine_del, spine_ins) = self.split(spine);
                MergeSpineSeqNode::OneDeleteConflict(del, Colored::new_white(spine_del), spine_ins)
            }
            (SpineSeqNode::DeleteConflict(c_del, c_ins), SpineSeqNode::Zipped(spine))
            | (SpineSeqNode::Zipped(spine), SpineSeqNode::DeleteConflict(c_del, c_ins)) => {
                let (spine_del, spine_ins) = self.split(spine);
                MergeSpineSeqNode::BothDeleteConflict(
                    Colored::new_white(spine_del),
                    spine_ins,
                    c_del,
                    c_ins,
                )
            }
            // Insertions are handled at the SpineSeq level, do not merge them here
            (SpineSeqNode::Inserted(_), _) | (_, SpineSeqNode::Inserted(_)) => unreachable!(),
            (SpineSeqNode::InsertOrderConflict(_), _)
            | (_, SpineSeqNode::InsertOrderConflict(_)) => unreachable!(),
        }
    }
}

impl<S, MS, D, I> Merge<SpineSeq<S, D, I>, SpineSeq<S, D, I>, MergeSpineSeq<MS, D, I>>
    for SpineAligner
where
    SpineAligner: Merge<SpineSeqNode<S, D, I>, SpineSeqNode<S, D, I>, MergeSpineSeqNode<MS, D, I>>,
{
    fn can_merge(&mut self, left: &SpineSeq<S, D, I>, right: &SpineSeq<S, D, I>) -> bool {
        // Insertions are never incompatible changes but they can create conflicts
        let is_not_insert = |e: &&SpineSeqNode<S, D, I>| match **e {
            SpineSeqNode::Inserted(_) | SpineSeqNode::InsertOrderConflict(_) => false,
            _ => true,
        };
        let left_list: Vec<_> = left.0.iter().filter(is_not_insert).collect();
        let right_list: Vec<_> = right.0.iter().filter(is_not_insert).collect();
        if left_list.len() != right_list.len() {
            // Cannot merge if sizes of the original lists differ
            return false;
        }
        left_list
            .into_iter()
            .zip(right_list)
            .all(|(left, right)| self.can_merge(left, right))
    }

    fn merge(
        &mut self,
        left: SpineSeq<S, D, I>,
        right: SpineSeq<S, D, I>,
    ) -> MergeSpineSeq<MS, D, I> {
        let mut left_iter = left.0.into_iter().peekable();
        let mut right_iter = right.0.into_iter().peekable();
        let mut merged_seq = Vec::new();

        let into_ins_list = |node| match node {
            SpineSeqNode::Inserted(ins) => vec![ins],
            SpineSeqNode::InsertOrderConflict(list) => list,
            _ => unreachable!(),
        };

        while left_iter.peek().is_some() || right_iter.peek().is_some() {
            merged_seq.push(match (left_iter.peek(), right_iter.peek()) {
                (Some(&SpineSeqNode::Inserted(_)), Some(&SpineSeqNode::Inserted(_)))
                | (
                    Some(&SpineSeqNode::InsertOrderConflict(_)),
                    Some(&SpineSeqNode::InsertOrderConflict(_)),
                )
                | (Some(&SpineSeqNode::Inserted(_)), Some(&SpineSeqNode::InsertOrderConflict(_)))
                | (Some(&SpineSeqNode::InsertOrderConflict(_)), Some(&SpineSeqNode::Inserted(_))) =>
                {
                    // Insertion in both sides, consume both
                    let mut ins_list = into_ins_list(left_iter.next().unwrap());
                    ins_list.extend(into_ins_list(right_iter.next().unwrap()));
                    MergeSpineSeqNode::Insert(ins_list)
                }
                // One side only is an insertion, output it and continue.
                (Some(&SpineSeqNode::Inserted(_)), _)
                | (Some(&SpineSeqNode::InsertOrderConflict(_)), _) => {
                    MergeSpineSeqNode::Insert(into_ins_list(left_iter.next().unwrap()))
                }
                (_, Some(&SpineSeqNode::Inserted(_)))
                | (_, Some(&SpineSeqNode::InsertOrderConflict(_))) => {
                    MergeSpineSeqNode::Insert(into_ins_list(right_iter.next().unwrap()))
                }
                _ => {
                    // No insertion operation in any argument, consume both
                    self.merge(left_iter.next().unwrap(), right_iter.next().unwrap())
                }
            })
        }

        MergeSpineSeq(merged_seq)
    }
}

impl<S, MS, D, I> Convert<SpineNode<S, D, I>, MergeSpineNode<MS, D, I>> for SpineAligner
where
    SpineAligner: Convert<S, MS>,
{
    fn convert(&mut self, input: SpineNode<S, D, I>) -> MergeSpineNode<MS, D, I> {
        match input {
            SpineNode::Spine(spine) => MergeSpineNode::Spine(self.convert(spine)),
            SpineNode::Unchanged => MergeSpineNode::Unchanged,
            SpineNode::Changed(del, ins) => MergeSpineNode::OneChange(del, ins),
        }
    }
}

impl<S, MS, D, I> Convert<SpineSeq<S, D, I>, MergeSpineSeq<MS, D, I>> for SpineAligner
where
    SpineAligner: Convert<SpineNode<S, D, I>, MergeSpineNode<MS, D, I>>,
{
    fn convert(&mut self, input: SpineSeq<S, D, I>) -> MergeSpineSeq<MS, D, I> {
        MergeSpineSeq(
            input
                .0
                .into_iter()
                .map(|node| match node {
                    SpineSeqNode::Zipped(node) => MergeSpineSeqNode::Zipped(self.convert(node)),
                    SpineSeqNode::Deleted(del) => {
                        let unchanged_mv = Metavariable(self.next_metavar);
                        self.next_metavar += 1;
                        let unchanged_del = Colored::new_white(DelNode::Ellided(unchanged_mv));
                        let unchanged_ins = InsNode::Ellided(Colored::new_white(unchanged_mv));
                        MergeSpineSeqNode::OneDeleteConflict(del, unchanged_del, unchanged_ins)
                    }
                    SpineSeqNode::DeleteConflict(conflict_del, conflict_ins) => {
                        let unchanged_mv = Metavariable(self.next_metavar);
                        self.next_metavar += 1;
                        let unchanged_del = Colored::new_white(DelNode::Ellided(unchanged_mv));
                        let unchanged_ins = InsNode::Ellided(Colored::new_white(unchanged_mv));
                        MergeSpineSeqNode::BothDeleteConflict(
                            conflict_del,
                            conflict_ins,
                            unchanged_del,
                            unchanged_ins,
                        )
                    }
                    SpineSeqNode::Inserted(ins_seq) => MergeSpineSeqNode::Insert(vec![ins_seq]),
                    SpineSeqNode::InsertOrderConflict(ins_conflict) => {
                        MergeSpineSeqNode::Insert(ins_conflict)
                    }
                })
                .collect(),
        )
    }
}

pub fn align_spine<I, O>(left: I, right: I, next_metavar: usize) -> Option<(O, usize)>
where
    SpineAligner: Merge<I, I, O>,
{
    let mut aligner = SpineAligner { next_metavar };
    if aligner.can_merge(&left, &right) {
        Some((aligner.merge(left, right), aligner.next_metavar))
    } else {
        None
    }
}
