use super::align_spine::{MergeSpineNode, MergeSpineSeq, MergeSpineSeqNode};
use super::{ColorSet, Colored, DelNode, InsNode, InsSeq, InsSeqNode};
use crate::family_traits::{Convert, Merge, VisitMut};
use std::any::Any;

pub enum ISpineNode<S, D, I> {
    Spine(S),
    Unchanged,
    OneChange(DelNode<D, I>, InsNode<I>),
    BothChanged(DelNode<D, I>, DelNode<D, I>, InsNode<I>),
}
pub enum ISpineSeqNode<S, D, I> {
    Zipped(ISpineNode<S, D, I>),
    BothDeleted(DelNode<D, I>, DelNode<D, I>),
    DeleteConflict(DelNode<D, I>, DelNode<D, I>, InsNode<I>),
    Insert(Vec<Colored<Vec<InsNode<I>>>>),
}
pub struct ISpineSeq<S, D, I>(pub Vec<ISpineSeqNode<S, D, I>>);

pub enum MetavarStatus {
    Keep,
    Replace(Box<dyn Any>),
    Conflict,
}

pub struct InsMerger {
    metavars_status: Vec<Option<MetavarStatus>>,
}

impl<I> Merge<InsNode<I>, InsNode<I>, InsNode<I>> for InsMerger
where
    InsMerger: Merge<Colored<I>, Colored<I>, Colored<I>>,
{
    fn can_merge(&mut self, _: &InsNode<I>, _: &InsNode<I>) -> bool {
        // InsNode's can always be merged by creating a conflict
        true
    }

    fn merge(&mut self, left: InsNode<I>, right: InsNode<I>) -> InsNode<I> {
        match (left, right) {
            (InsNode::InPlace(left_subtree), InsNode::InPlace(right_subtree))
                if self.can_merge(&left_subtree, &right_subtree) =>
            {
                InsNode::InPlace(self.merge(left_subtree, right_subtree))
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
}

impl<I> Merge<InsSeq<I>, InsSeq<I>, InsSeq<I>> for InsMerger
where
    InsMerger: Merge<InsNode<I>, InsNode<I>, InsNode<I>>,
{
    fn can_merge(&mut self, left: &InsSeq<I>, right: &InsSeq<I>) -> bool {
        // Only lists of the same size
        if left.0.len() != right.0.len() {
            return false;
        }

        // Without insert order conflicts
        left.0.iter().zip(&right.0).all(|pair| match pair {
            (InsSeqNode::InsertOrderConflict(_), _) | (_, InsSeqNode::InsertOrderConflict(_)) => {
                false
            }
            _ => true,
        })
    }

    fn merge(&mut self, left: InsSeq<I>, right: InsSeq<I>) -> InsSeq<I> {
        InsSeq(
            left.0
                .into_iter()
                .zip(right.0)
                .map(|pair| match pair {
                    (InsSeqNode::Node(left), InsSeqNode::Node(right)) => {
                        InsSeqNode::Node(self.merge(left, right))
                    }
                    (InsSeqNode::DeleteConflict(left), InsSeqNode::DeleteConflict(right))
                    | (InsSeqNode::DeleteConflict(left), InsSeqNode::Node(right))
                    | (InsSeqNode::Node(left), InsSeqNode::DeleteConflict(right)) => {
                        InsSeqNode::DeleteConflict(self.merge(left, right))
                    }
                    (InsSeqNode::InsertOrderConflict(_), _)
                    | (_, InsSeqNode::InsertOrderConflict(_)) => {
                        panic!("Trying to merge insert tree sequences with order conflicts")
                    }
                })
                .collect(),
        )
    }
}

impl<D, I> Merge<DelNode<D, I>, InsNode<I>, DelNode<D, I>> for InsMerger
where
    InsMerger: Merge<D, I, D>,
    InsMerger: Merge<I, I, I>,
    InsNode<I>: Clone + 'static,
{
    fn can_merge(&mut self, del: &DelNode<D, I>, ins: &InsNode<I>) -> bool {
        match (del, ins) {
            (DelNode::Ellided(_), _) => true,
            (DelNode::MetavariableConflict(_, _, _), _) => true,
            (DelNode::InPlace(del_subtree), InsNode::InPlace(ins_subtree)) => {
                if ins_subtree.colors == ColorSet::white() {
                    self.can_merge(&del_subtree.node, &ins_subtree.node)
                } else {
                    // I don't really see how this branch can be triggered on real examples,
                    // but refuse to merge for color preservation
                    false
                }
            }
            _ => false,
        }
    }

    fn merge(&mut self, del: DelNode<D, I>, ins: InsNode<I>) -> DelNode<D, I> {
        match (del, ins) {
            (DelNode::Ellided(mv), ins) => {
                // Here we may have to clone the insert tree once to check for potential conflicts
                let mv_id = mv.node.0;
                let cur_status = std::mem::take(&mut self.metavars_status[mv_id]);
                self.metavars_status[mv_id] = Some(match cur_status {
                    None => MetavarStatus::Replace(Box::new(vec![ins.clone()])),
                    Some(MetavarStatus::Replace(mut repl_ins)) => {
                        let repl_ins_list: &mut Vec<_> = repl_ins.downcast_mut().unwrap();
                        repl_ins_list.push(ins.clone());
                        MetavarStatus::Replace(repl_ins)
                    }
                    Some(MetavarStatus::Keep) | Some(MetavarStatus::Conflict) => {
                        MetavarStatus::Conflict
                    }
                });
                DelNode::MetavariableConflict(mv.node, Box::new(DelNode::Ellided(mv)), ins)
            }
            (DelNode::MetavariableConflict(mv, del, conflict_ins), ins) => {
                self.metavars_status[mv.0] = Some(MetavarStatus::Conflict);
                DelNode::MetavariableConflict(mv, del, self.merge(conflict_ins, ins))
            }
            (DelNode::InPlace(del_subtree), InsNode::InPlace(ins_subtree)) => {
                DelNode::InPlace(Colored {
                    node: self.merge(del_subtree.node, ins_subtree.node),
                    colors: del_subtree.colors,
                })
            }
            _ => panic!("<Merge<del, ins, del>>::merge() called with incompatible data"),
        }
    }
}

impl<D, I> Merge<Vec<DelNode<D, I>>, InsSeq<I>, Vec<DelNode<D, I>>> for InsMerger
where
    InsMerger: Merge<DelNode<D, I>, InsNode<I>, DelNode<D, I>>,
{
    fn can_merge(&mut self, del_seq: &Vec<DelNode<D, I>>, ins_seq: &InsSeq<I>) -> bool {
        if del_seq.len() != ins_seq.0.len() {
            return false;
        }

        del_seq
            .iter()
            .zip(&ins_seq.0)
            .all(|(del, ins_seq_node)| match ins_seq_node {
                InsSeqNode::Node(ins) => self.can_merge(del, ins),
                _ => false,
            })
    }

    fn merge(&mut self, del_seq: Vec<DelNode<D, I>>, ins_seq: InsSeq<I>) -> Vec<DelNode<D, I>> {
        del_seq
            .into_iter()
            .zip(ins_seq.0)
            .map(|(del, ins_seq_node)| match ins_seq_node {
                InsSeqNode::Node(ins) => self.merge(del, ins),
                _ => panic!("InsSeq contains a conflict when merged with a deletion tree"),
            })
            .collect()
    }
}

impl<D, I> VisitMut<DelNode<D, I>> for InsMerger
where
    InsMerger: VisitMut<D>,
{
    fn visit_mut(&mut self, del: &mut DelNode<D, I>) {
        match del {
            DelNode::InPlace(del_subtree) => self.visit_mut(&mut del_subtree.node),
            DelNode::Ellided(mv) => {
                let mv_id = mv.node.0;
                self.metavars_status[mv_id] = Some(match &self.metavars_status[mv_id] {
                    None | Some(MetavarStatus::Keep) => MetavarStatus::Keep,
                    Some(MetavarStatus::Replace(_)) | Some(MetavarStatus::Conflict) => {
                        MetavarStatus::Conflict
                    }
                });
                *del = DelNode::MetavariableConflict(
                    mv.node,
                    Box::new(DelNode::Ellided(*mv)),
                    InsNode::Ellided(Colored::new_white(mv.node)),
                )
            }
            DelNode::MetavariableConflict(mv, del, _) => {
                self.metavars_status[mv.0] = Some(MetavarStatus::Conflict);
                VisitMut::<DelNode<D, I>>::visit_mut(self, del)
            }
        }
    }
}

impl<MS, IS, D, I> Convert<MergeSpineNode<MS, D, I>, ISpineNode<IS, D, I>> for InsMerger
where
    InsMerger: Convert<MS, IS>,
    InsMerger: Merge<DelNode<D, I>, InsNode<I>, DelNode<D, I>>,
    InsMerger: Merge<InsNode<I>, InsNode<I>, InsNode<I>>,
    InsMerger: VisitMut<DelNode<D, I>>,
{
    fn convert(&mut self, node: MergeSpineNode<MS, D, I>) -> ISpineNode<IS, D, I> {
        match node {
            MergeSpineNode::Spine(spine) => ISpineNode::Spine(self.convert(spine)),
            MergeSpineNode::Unchanged => ISpineNode::Unchanged,
            MergeSpineNode::OneChange(mut del, ins) => {
                self.visit_mut(&mut del);
                ISpineNode::OneChange(del, ins)
            }
            MergeSpineNode::BothChanged(mut left_del, left_ins, mut right_del, right_ins) => {
                match (
                    self.can_merge(&right_del, &left_ins),
                    self.can_merge(&left_del, &right_ins),
                ) {
                    (true, true) | (false, false) => {
                        self.visit_mut(&mut left_del);
                        self.visit_mut(&mut right_del);
                        ISpineNode::BothChanged(
                            left_del,
                            right_del,
                            self.merge(left_ins, right_ins),
                        )
                    }
                    (true, false) => {
                        let right_del = self.merge(right_del, left_ins);
                        self.visit_mut(&mut left_del);
                        ISpineNode::BothChanged(right_del, left_del, right_ins)
                    }
                    (false, true) => {
                        let left_del = self.merge(left_del, right_ins);
                        self.visit_mut(&mut right_del);
                        ISpineNode::BothChanged(left_del, right_del, left_ins)
                    }
                }
            }
        }
    }
}

impl<MS, IS, D, I> Convert<MergeSpineSeq<MS, D, I>, ISpineSeq<IS, D, I>> for InsMerger
where
    InsMerger: Convert<MergeSpineNode<MS, D, I>, ISpineNode<IS, D, I>>,
    InsMerger: Merge<DelNode<D, I>, InsNode<I>, DelNode<D, I>>,
    InsMerger: Merge<InsNode<I>, InsNode<I>, InsNode<I>>,
    InsMerger: VisitMut<DelNode<D, I>>,
{
    fn convert(&mut self, seq: MergeSpineSeq<MS, D, I>) -> ISpineSeq<IS, D, I> {
        ISpineSeq(
            seq.0
                .into_iter()
                .map(|node| match node {
                    MergeSpineSeqNode::Zipped(node) => ISpineSeqNode::Zipped(self.convert(node)),
                    MergeSpineSeqNode::BothDeleted(mut left_del, mut right_del) => {
                        self.visit_mut(&mut left_del);
                        self.visit_mut(&mut right_del);
                        ISpineSeqNode::BothDeleted(left_del, right_del)
                    }
                    MergeSpineSeqNode::OneDeleteConflict(
                        mut del,
                        mut conflict_del,
                        conflict_ins,
                    ) => {
                        if self.can_merge(&del, &conflict_ins) {
                            self.visit_mut(&mut conflict_del);
                            ISpineSeqNode::BothDeleted(self.merge(del, conflict_ins), conflict_del)
                        } else {
                            self.visit_mut(&mut del);
                            self.visit_mut(&mut conflict_del);
                            ISpineSeqNode::DeleteConflict(del, conflict_del, conflict_ins)
                        }
                    }
                    MergeSpineSeqNode::BothDeleteConflict(
                        mut left_del,
                        left_ins,
                        mut right_del,
                        right_ins,
                    ) => {
                        match (
                            self.can_merge(&right_del, &left_ins),
                            self.can_merge(&left_del, &right_ins),
                        ) {
                            (false, false) => {
                                self.visit_mut(&mut left_del);
                                self.visit_mut(&mut right_del);
                                ISpineSeqNode::DeleteConflict(
                                    left_del,
                                    right_del,
                                    self.merge(left_ins, right_ins),
                                )
                            }
                            (true, false) => {
                                self.visit_mut(&mut left_del);
                                ISpineSeqNode::DeleteConflict(
                                    self.merge(right_del, left_ins),
                                    left_del,
                                    right_ins,
                                )
                            }
                            (false, true) => {
                                self.visit_mut(&mut right_del);
                                ISpineSeqNode::DeleteConflict(
                                    self.merge(left_del, right_ins),
                                    right_del,
                                    left_ins,
                                )
                            }
                            (true, true) => {
                                // Solve both conflicts at once!
                                ISpineSeqNode::BothDeleted(
                                    self.merge(left_del, right_ins),
                                    self.merge(right_del, left_ins),
                                )
                            }
                        }
                    }
                    MergeSpineSeqNode::Insert(ins_vec) => ISpineSeqNode::Insert(ins_vec),
                })
                .collect(),
        )
    }
}

pub fn merge_ins<I, O>(input: I, nb_vars: usize) -> (O, Vec<MetavarStatus>)
where
    InsMerger: Convert<I, O>,
{
    let mut merger = InsMerger {
        metavars_status: Vec::new(),
    };
    merger.metavars_status.resize_with(nb_vars, || None);
    let output = merger.convert(input);
    (
        output,
        merger
            .metavars_status
            .into_iter()
            .collect::<Option<Vec<MetavarStatus>>>()
            .expect("Found a metavariable without status"),
    )
}
