use super::metavar_renamer::MetavarRenamer;
use super::{Colored, DelNode, InsNode, InsSeq, InsSeqNode, SpineNode, SpineSeq, SpineSeqNode};
use crate::diff_tree::Metavariable;
use crate::family_traits::{Convert, Merge, Split, VisitMut};

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

pub struct SpineSplitter<'a> {
    new_metavars: &'a mut Vec<Option<Metavariable>>,
    next_metavar: &'a mut usize,
}

impl<'a> SpineSplitter<'a> {
    fn renamer(&mut self) -> MetavarRenamer {
        MetavarRenamer {
            new_metavars: &mut *self.new_metavars,
            next_metavar: &mut *self.next_metavar,
        }
    }
}

impl<'a, S, D, I> Split<SpineNode<S, D, I>, DelNode<D, I>, InsNode<I>> for SpineSplitter<'a>
where
    SpineSplitter<'a>: Split<S, D, I>,
    for<'b> MetavarRenamer<'b>: VisitMut<DelNode<D, I>>,
    for<'b> MetavarRenamer<'b>: VisitMut<InsNode<I>>,
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
                let new_metavar = Metavariable(*self.next_metavar);
                *self.next_metavar += 1;
                (
                    DelNode::Ellided(new_metavar),
                    InsNode::Ellided(Colored::new_white(new_metavar)),
                )
            }
            SpineNode::Changed(mut del, mut ins) => {
                self.renamer().visit_mut(&mut del);
                self.renamer().visit_mut(&mut ins);
                (del, ins)
            }
        }
    }
}

impl<'a, S, D, I> Split<SpineSeq<S, D, I>, Vec<DelNode<D, I>>, InsSeq<I>> for SpineSplitter<'a>
where
    SpineSplitter<'a>: Split<SpineNode<S, D, I>, DelNode<D, I>, InsNode<I>>,
    for<'b> MetavarRenamer<'b>: VisitMut<DelNode<D, I>>,
    for<'b> MetavarRenamer<'b>: VisitMut<InsNode<I>>,
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
                SpineSeqNode::Deleted(mut del) => {
                    self.renamer().visit_mut(&mut del);
                    del_seq.push(del.node);
                }
                SpineSeqNode::DeleteConflict(mut del, mut ins) => {
                    self.renamer().visit_mut(&mut del);
                    self.renamer().visit_mut(&mut ins);
                    del_seq.push(del.node);
                    ins_seq.push(InsSeqNode::DeleteConflict(ins));
                }
                SpineSeqNode::Inserted(ins_list) => {
                    for mut ins in ins_list.node {
                        self.renamer().visit_mut(&mut ins);
                        ins_seq.push(InsSeqNode::Node(ins));
                    }
                }
                SpineSeqNode::InsertOrderConflict(ins_conflict) => {
                    let mut conflict = InsSeqNode::InsertOrderConflict(ins_conflict);
                    self.renamer().visit_mut(&mut conflict);
                    ins_seq.push(conflict);
                }
            }
        }
        (del_seq, InsSeq(ins_seq))
    }
}

pub struct SpineMerger {
    left_new_metavars: Vec<Option<Metavariable>>,
    right_new_metavars: Vec<Option<Metavariable>>,
    next_metavar: usize,
}

impl SpineMerger {
    fn left_splitter(&mut self) -> SpineSplitter {
        SpineSplitter {
            new_metavars: &mut self.left_new_metavars,
            next_metavar: &mut self.next_metavar,
        }
    }

    fn right_splitter(&mut self) -> SpineSplitter {
        SpineSplitter {
            new_metavars: &mut self.right_new_metavars,
            next_metavar: &mut self.next_metavar,
        }
    }
}

impl<S, MS, D, I> Merge<SpineNode<S, D, I>, SpineNode<S, D, I>, MergeSpineNode<MS, D, I>>
    for SpineMerger
where
    SpineMerger: Merge<S, S, MS>,
    for<'a> SpineSplitter<'a>: Split<S, D, I>,
    for<'a> MetavarRenamer<'a>: VisitMut<D>,
    for<'a> MetavarRenamer<'a>: VisitMut<I>,
    for<'a> UnchangedMerger<'a>: Convert<S, MS>,
{
    fn can_merge(&mut self, left: &SpineNode<S, D, I>, right: &SpineNode<S, D, I>) -> bool {
        // In SpineMerger we only check matching spines. More checks will be performed later.
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
            (SpineNode::Spine(spine), SpineNode::Unchanged) => {
                let new_spine = UnchangedMerger(self.left_splitter().renamer()).convert(spine);
                MergeSpineNode::Spine(new_spine)
            }
            (SpineNode::Unchanged, SpineNode::Spine(spine)) => {
                let new_spine = UnchangedMerger(self.right_splitter().renamer()).convert(spine);
                MergeSpineNode::Spine(new_spine)
            }
            (SpineNode::Changed(mut del, mut ins), SpineNode::Unchanged) => {
                self.left_splitter().renamer().visit_mut(&mut del);
                self.left_splitter().renamer().visit_mut(&mut ins);
                MergeSpineNode::OneChange(del, ins)
            }
            (SpineNode::Unchanged, SpineNode::Changed(mut del, mut ins)) => {
                self.right_splitter().renamer().visit_mut(&mut del);
                self.right_splitter().renamer().visit_mut(&mut ins);
                MergeSpineNode::OneChange(del, ins)
            }
            (
                SpineNode::Changed(mut left_del, mut left_ins),
                SpineNode::Changed(mut right_del, mut right_ins),
            ) => {
                self.left_splitter().renamer().visit_mut(&mut left_del);
                self.left_splitter().renamer().visit_mut(&mut left_ins);
                self.right_splitter().renamer().visit_mut(&mut right_del);
                self.right_splitter().renamer().visit_mut(&mut right_ins);
                MergeSpineNode::BothChanged(left_del, left_ins, right_del, right_ins)
            }
            (SpineNode::Spine(spine), SpineNode::Changed(mut right_del, mut right_ins)) => {
                let (left_del, left_ins) = self.left_splitter().split(spine);
                let left_del = DelNode::InPlace(left_del);
                let left_ins = InsNode::InPlace(Colored::new_white(left_ins));
                self.right_splitter().renamer().visit_mut(&mut right_del);
                self.right_splitter().renamer().visit_mut(&mut right_ins);
                MergeSpineNode::BothChanged(left_del, left_ins, right_del, right_ins)
            }
            (SpineNode::Changed(mut left_del, mut left_ins), SpineNode::Spine(spine)) => {
                self.left_splitter().renamer().visit_mut(&mut left_del);
                self.left_splitter().renamer().visit_mut(&mut left_ins);
                let (right_del, right_ins) = self.right_splitter().split(spine);
                let right_del = DelNode::InPlace(right_del);
                let right_ins = InsNode::InPlace(Colored::new_white(right_ins));
                MergeSpineNode::BothChanged(left_del, left_ins, right_del, right_ins)
            }
        }
    }
}

impl<S, MS, D, I> Merge<SpineSeqNode<S, D, I>, SpineSeqNode<S, D, I>, MergeSpineSeqNode<MS, D, I>>
    for SpineMerger
where
    SpineMerger: Merge<SpineNode<S, D, I>, SpineNode<S, D, I>, MergeSpineNode<MS, D, I>>,
    for<'a> SpineSplitter<'a>: Split<SpineNode<S, D, I>, DelNode<D, I>, InsNode<I>>,
    for<'a> MetavarRenamer<'a>: VisitMut<DelNode<D, I>>,
    for<'a> MetavarRenamer<'a>: VisitMut<InsNode<I>>,
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
            (SpineSeqNode::Deleted(mut left_del), SpineSeqNode::Deleted(mut right_del)) => {
                self.left_splitter().renamer().visit_mut(&mut left_del);
                self.left_splitter().renamer().visit_mut(&mut right_del);
                MergeSpineSeqNode::BothDeleted(left_del, right_del)
            }
            (
                SpineSeqNode::DeleteConflict(mut left_del, mut left_ins),
                SpineSeqNode::Deleted(mut right_del),
            ) => {
                self.right_splitter().renamer().visit_mut(&mut right_del);
                self.left_splitter().renamer().visit_mut(&mut left_del);
                self.left_splitter().renamer().visit_mut(&mut left_ins);
                MergeSpineSeqNode::OneDeleteConflict(right_del, left_del, left_ins)
            }
            (
                SpineSeqNode::Deleted(mut left_del),
                SpineSeqNode::DeleteConflict(mut right_del, mut right_ins),
            ) => {
                self.left_splitter().renamer().visit_mut(&mut left_del);
                self.right_splitter().renamer().visit_mut(&mut right_del);
                self.right_splitter().renamer().visit_mut(&mut right_ins);
                MergeSpineSeqNode::OneDeleteConflict(left_del, right_del, right_ins)
            }
            (
                SpineSeqNode::DeleteConflict(mut left_del, mut left_ins),
                SpineSeqNode::DeleteConflict(mut right_del, mut right_ins),
            ) => {
                self.left_splitter().renamer().visit_mut(&mut left_del);
                self.left_splitter().renamer().visit_mut(&mut left_ins);
                self.right_splitter().renamer().visit_mut(&mut right_del);
                self.right_splitter().renamer().visit_mut(&mut right_ins);
                MergeSpineSeqNode::BothDeleteConflict(left_del, left_ins, right_del, right_ins)
            }
            (SpineSeqNode::Zipped(left_spine), SpineSeqNode::Deleted(mut right_del)) => {
                self.right_splitter().renamer().visit_mut(&mut right_del);
                let (left_del, left_ins) = self.left_splitter().split(left_spine);
                MergeSpineSeqNode::OneDeleteConflict(
                    right_del,
                    Colored::new_white(left_del),
                    left_ins,
                )
            }
            (SpineSeqNode::Deleted(mut left_del), SpineSeqNode::Zipped(right_spine)) => {
                self.left_splitter().renamer().visit_mut(&mut left_del);
                let (right_del, right_ins) = self.right_splitter().split(right_spine);
                MergeSpineSeqNode::OneDeleteConflict(
                    left_del,
                    Colored::new_white(right_del),
                    right_ins,
                )
            }
            (
                SpineSeqNode::Zipped(left_spine),
                SpineSeqNode::DeleteConflict(mut right_del, mut right_ins),
            ) => {
                let (left_del, left_ins) = self.left_splitter().split(left_spine);
                self.right_splitter().renamer().visit_mut(&mut right_del);
                self.right_splitter().renamer().visit_mut(&mut right_ins);
                MergeSpineSeqNode::BothDeleteConflict(
                    Colored::new_white(left_del),
                    left_ins,
                    right_del,
                    right_ins,
                )
            }
            (
                SpineSeqNode::DeleteConflict(mut left_del, mut left_ins),
                SpineSeqNode::Zipped(right_spine),
            ) => {
                self.left_splitter().renamer().visit_mut(&mut left_del);
                self.left_splitter().renamer().visit_mut(&mut left_ins);
                let (right_del, right_ins) = self.right_splitter().split(right_spine);
                MergeSpineSeqNode::BothDeleteConflict(
                    left_del,
                    left_ins,
                    Colored::new_white(right_del),
                    right_ins,
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
    for SpineMerger
where
    SpineMerger: Merge<SpineSeqNode<S, D, I>, SpineSeqNode<S, D, I>, MergeSpineSeqNode<MS, D, I>>,
    for<'a> MetavarRenamer<'a>: VisitMut<InsNode<I>>,
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

        let into_ins_list = |node, mut renamer: MetavarRenamer| match node {
            SpineSeqNode::Inserted(mut ins) => {
                for ins_node in &mut ins.node {
                    renamer.visit_mut(ins_node);
                }
                vec![ins]
            }
            SpineSeqNode::InsertOrderConflict(mut list) => {
                for ins_list in &mut list {
                    for ins_node in &mut ins_list.node {
                        renamer.visit_mut(ins_node);
                    }
                }
                list
            }
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
                    let mut ins_list =
                        into_ins_list(left_iter.next().unwrap(), self.left_splitter().renamer());
                    ins_list.extend(into_ins_list(
                        right_iter.next().unwrap(),
                        self.right_splitter().renamer(),
                    ));
                    MergeSpineSeqNode::Insert(ins_list)
                }
                // One side only is an insertion, output it and continue.
                (Some(&SpineSeqNode::Inserted(_)), _)
                | (Some(&SpineSeqNode::InsertOrderConflict(_)), _) => MergeSpineSeqNode::Insert(
                    into_ins_list(left_iter.next().unwrap(), self.left_splitter().renamer()),
                ),
                (_, Some(&SpineSeqNode::Inserted(_)))
                | (_, Some(&SpineSeqNode::InsertOrderConflict(_))) => MergeSpineSeqNode::Insert(
                    into_ins_list(right_iter.next().unwrap(), self.right_splitter().renamer()),
                ),
                _ => {
                    // No insertion operation in any argument, consume both
                    self.merge(left_iter.next().unwrap(), right_iter.next().unwrap())
                }
            })
        }

        MergeSpineSeq(merged_seq)
    }
}

pub struct UnchangedMerger<'a>(MetavarRenamer<'a>);

impl<'a, S, MS, D, I> Convert<SpineNode<S, D, I>, MergeSpineNode<MS, D, I>> for UnchangedMerger<'a>
where
    UnchangedMerger<'a>: Convert<S, MS>,
    MetavarRenamer<'a>: VisitMut<DelNode<D, I>>,
    MetavarRenamer<'a>: VisitMut<InsNode<I>>,
{
    fn convert(&mut self, input: SpineNode<S, D, I>) -> MergeSpineNode<MS, D, I> {
        match input {
            SpineNode::Spine(spine) => MergeSpineNode::Spine(self.convert(spine)),
            SpineNode::Unchanged => MergeSpineNode::Unchanged,
            SpineNode::Changed(mut del, mut ins) => {
                self.0.visit_mut(&mut del);
                self.0.visit_mut(&mut ins);
                MergeSpineNode::OneChange(del, ins)
            }
        }
    }
}

impl<'a, S, MS, D, I> Convert<SpineSeq<S, D, I>, MergeSpineSeq<MS, D, I>> for UnchangedMerger<'a>
where
    UnchangedMerger<'a>: Convert<SpineNode<S, D, I>, MergeSpineNode<MS, D, I>>,
    MetavarRenamer<'a>: VisitMut<DelNode<D, I>>,
    MetavarRenamer<'a>: VisitMut<InsNode<I>>,
{
    fn convert(&mut self, input: SpineSeq<S, D, I>) -> MergeSpineSeq<MS, D, I> {
        MergeSpineSeq(
            input
                .0
                .into_iter()
                .map(|node| match node {
                    SpineSeqNode::Zipped(node) => MergeSpineSeqNode::Zipped(self.convert(node)),
                    SpineSeqNode::Deleted(mut del) => {
                        self.0.visit_mut(&mut del);
                        let unchanged_mv = Metavariable(*self.0.next_metavar);
                        *self.0.next_metavar += 1;
                        let unchanged_del = Colored::new_white(DelNode::Ellided(unchanged_mv));
                        let unchanged_ins = InsNode::Ellided(Colored::new_white(unchanged_mv));
                        MergeSpineSeqNode::OneDeleteConflict(del, unchanged_del, unchanged_ins)
                    }
                    SpineSeqNode::DeleteConflict(mut conflict_del, mut conflict_ins) => {
                        self.0.visit_mut(&mut conflict_del);
                        self.0.visit_mut(&mut conflict_ins);
                        let unchanged_mv = Metavariable(*self.0.next_metavar);
                        *self.0.next_metavar += 1;
                        let unchanged_del = Colored::new_white(DelNode::Ellided(unchanged_mv));
                        let unchanged_ins = InsNode::Ellided(Colored::new_white(unchanged_mv));
                        MergeSpineSeqNode::BothDeleteConflict(
                            conflict_del,
                            conflict_ins,
                            unchanged_del,
                            unchanged_ins,
                        )
                    }
                    SpineSeqNode::Inserted(mut ins_seq) => {
                        for ins_node in &mut ins_seq.node {
                            self.0.visit_mut(ins_node)
                        }
                        MergeSpineSeqNode::Insert(vec![ins_seq])
                    }
                    SpineSeqNode::InsertOrderConflict(mut ins_conflict) => {
                        for ins_seq in &mut ins_conflict {
                            for ins_node in &mut ins_seq.node {
                                self.0.visit_mut(ins_node)
                            }
                        }
                        MergeSpineSeqNode::Insert(ins_conflict)
                    }
                })
                .collect(),
        )
    }
}

pub fn align_spine<I, O>(left: I, right: I) -> Option<(O, usize)>
where
    SpineMerger: Merge<I, I, O>,
{
    let mut merger = SpineMerger {
        left_new_metavars: Vec::new(),
        right_new_metavars: Vec::new(),
        next_metavar: 0,
    };
    if merger.can_merge(&left, &right) {
        Some((merger.merge(left, right), merger.next_metavar))
    } else {
        None
    }
}
