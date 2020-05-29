use super::align_spine::{MergeSpineNode, MergeSpineSeq, MergeSpineSeqNode};
use super::{Colored, DelNode, InsNode, InsSeq, InsSeqNode};
use crate::family_traits::{Convert, Merge, Visit};
use std::any::Any;

pub enum ISpineNode<S, D, I> {
    Spine(S),
    Unchanged,
    OneChange(DelNode<D, I>, InsNode<I>),
    BothChanged(DelNode<D, I>, DelNode<D, I>, InsNode<I>),
}
pub enum ISpineSeqNode<S, D, I> {
    Zipped(ISpineNode<S, D, I>),
    BothDeleted(Colored<DelNode<D, I>>, Colored<DelNode<D, I>>),
    DeleteConflict(Colored<DelNode<D, I>>, Colored<DelNode<D, I>>, InsNode<I>),
    Insert(Vec<Colored<Vec<InsNode<I>>>>),
}
pub struct ISpineSeq<S, D, I>(pub Vec<ISpineSeqNode<S, D, I>>);

pub struct ColorMerger;

impl<I> Merge<InsNode<I>, InsNode<I>, InsNode<I>> for ColorMerger
where
    ColorMerger: Merge<Colored<I>, Colored<I>, Colored<I>>,
{
    fn can_merge(&mut self, left: &InsNode<I>, right: &InsNode<I>) -> bool {
        match (left, right) {
            (InsNode::InPlace(left), InsNode::InPlace(right)) => self.can_merge(left, right),
            (InsNode::Ellided(left), InsNode::Ellided(right)) => left.node == right.node,
            (InsNode::Conflict(left), InsNode::Conflict(right)) => {
                <ColorMerger as Merge<Vec<InsNode<I>>, _, _>>::can_merge(self, left, right)
            }
            _ => false,
        }
    }

    fn merge(&mut self, left: InsNode<I>, right: InsNode<I>) -> InsNode<I> {
        match (left, right) {
            (InsNode::InPlace(left), InsNode::InPlace(right)) => {
                InsNode::InPlace(self.merge(left, right))
            }
            (InsNode::Ellided(left), InsNode::Ellided(right)) => {
                let mut colors = left.colors;
                colors.extend(right.colors);
                InsNode::Ellided(Colored {
                    node: left.node,
                    colors,
                })
            }
            (InsNode::Conflict(left), InsNode::Conflict(right)) => InsNode::Conflict(
                <ColorMerger as Merge<Vec<InsNode<I>>, _, _>>::merge(self, left, right),
            ),
            _ => panic!("ColorMerger called on conflicting insertions"),
        }
    }
}

impl<I> Merge<InsSeq<I>, InsSeq<I>, InsSeq<I>> for ColorMerger
where
    ColorMerger: Merge<InsNode<I>, InsNode<I>, InsNode<I>>,
    ColorMerger: Merge<
        Vec<Colored<Vec<InsNode<I>>>>,
        Vec<Colored<Vec<InsNode<I>>>>,
        Vec<Colored<Vec<InsNode<I>>>>,
    >,
{
    fn can_merge(&mut self, left: &InsSeq<I>, right: &InsSeq<I>) -> bool {
        if left.0.len() != right.0.len() {
            return false;
        }
        left.0.iter().zip(&right.0).all(|pair| match pair {
            (InsSeqNode::Node(left), InsSeqNode::Node(right)) => self.can_merge(left, right),
            (InsSeqNode::DeleteConflict(left), InsSeqNode::DeleteConflict(right)) => {
                self.can_merge(left, right)
            }
            (InsSeqNode::InsertOrderConflict(left), InsSeqNode::InsertOrderConflict(right)) => {
                self.can_merge(left, right)
            }
            _ => false,
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
                    (InsSeqNode::DeleteConflict(left), InsSeqNode::DeleteConflict(right)) => {
                        InsSeqNode::DeleteConflict(self.merge(left, right))
                    }
                    (
                        InsSeqNode::InsertOrderConflict(left),
                        InsSeqNode::InsertOrderConflict(right),
                    ) => InsSeqNode::InsertOrderConflict(self.merge(left, right)),
                    _ => panic!("ColorMerger called on conflicting insertions"),
                })
                .collect(),
        )
    }
}

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
    ColorMerger: Merge<InsNode<I>, InsNode<I>, InsNode<I>>,
{
    fn can_merge(&mut self, del: &DelNode<D, I>, ins: &InsNode<I>) -> bool {
        match (del, ins) {
            (DelNode::Ellided(_), _) => true,
            (DelNode::MetavariableConflict(_, _, _), _) => true,
            (DelNode::InPlace(del_subtree), InsNode::InPlace(ins_subtree)) => {
                if ins_subtree.colors.is_empty() {
                    self.can_merge(del_subtree, &ins_subtree.node)
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
                let cur_status = std::mem::take(&mut self.metavars_status[mv.0]);
                self.metavars_status[mv.0] = Some(match cur_status {
                    None => MetavarStatus::Replace(Box::new(ins.clone())),
                    Some(MetavarStatus::Replace(other_ins)) => {
                        let other_ins = *other_ins.downcast::<InsNode<I>>().unwrap();
                        if ColorMerger.can_merge(&ins, &other_ins) {
                            MetavarStatus::Replace(Box::new(
                                ColorMerger.merge(ins.clone(), other_ins),
                            ))
                        } else {
                            MetavarStatus::Conflict
                        }
                    }
                    Some(MetavarStatus::Keep) | Some(MetavarStatus::Conflict) => {
                        MetavarStatus::Conflict
                    }
                });
                DelNode::MetavariableConflict(mv, Box::new(DelNode::Ellided(mv)), ins)
            }
            (DelNode::MetavariableConflict(mv, del, conflict_ins), ins) => {
                self.metavars_status[mv.0] = Some(MetavarStatus::Conflict);
                DelNode::MetavariableConflict(mv, del, self.merge(conflict_ins, ins))
            }
            (DelNode::InPlace(del_subtree), InsNode::InPlace(ins_subtree)) => {
                DelNode::InPlace(self.merge(del_subtree, ins_subtree.node))
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

impl<D, I> Visit<DelNode<D, I>> for InsMerger
where
    InsMerger: Visit<D>,
{
    fn visit(&mut self, del: &DelNode<D, I>) {
        match del {
            DelNode::InPlace(del_subtree) => self.visit(del_subtree),
            DelNode::Ellided(mv) => {
                self.metavars_status[mv.0] = Some(match &self.metavars_status[mv.0] {
                    None | Some(MetavarStatus::Keep) => MetavarStatus::Keep,
                    Some(MetavarStatus::Replace(_)) | Some(MetavarStatus::Conflict) => {
                        MetavarStatus::Conflict
                    }
                })
            }
            DelNode::MetavariableConflict(mv, del, _) => {
                self.metavars_status[mv.0] = Some(MetavarStatus::Conflict);
                <InsMerger as Visit<DelNode<D, I>>>::visit(self, del)
            }
        }
    }
}

impl<MS, IS, D, I> Convert<MergeSpineNode<MS, D, I>, ISpineNode<IS, D, I>> for InsMerger
where
    InsMerger: Convert<MS, IS>,
    InsMerger: Merge<DelNode<D, I>, InsNode<I>, DelNode<D, I>>,
    InsMerger: Merge<InsNode<I>, InsNode<I>, InsNode<I>>,
    InsMerger: Visit<DelNode<D, I>>,
{
    fn convert(&mut self, node: MergeSpineNode<MS, D, I>) -> ISpineNode<IS, D, I> {
        match node {
            MergeSpineNode::Spine(spine) => ISpineNode::Spine(self.convert(spine)),
            MergeSpineNode::Unchanged => ISpineNode::Unchanged,
            MergeSpineNode::OneChange(del, ins) => {
                self.visit(&del);
                ISpineNode::OneChange(del, ins)
            }
            MergeSpineNode::BothChanged(left_del, left_ins, right_del, right_ins) => {
                match (
                    self.can_merge(&right_del, &left_ins),
                    self.can_merge(&left_del, &right_ins),
                ) {
                    (true, true) | (false, false) => {
                        self.visit(&left_del);
                        self.visit(&right_del);
                        ISpineNode::BothChanged(
                            left_del,
                            right_del,
                            self.merge(left_ins, right_ins),
                        )
                    }
                    (true, false) => {
                        let right_del = self.merge(right_del, left_ins);
                        self.visit(&left_del);
                        ISpineNode::BothChanged(right_del, left_del, right_ins)
                    }
                    (false, true) => {
                        let left_del = self.merge(left_del, right_ins);
                        self.visit(&right_del);
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
    InsMerger: Visit<DelNode<D, I>>,
{
    fn convert(&mut self, seq: MergeSpineSeq<MS, D, I>) -> ISpineSeq<IS, D, I> {
        ISpineSeq(
            seq.0
                .into_iter()
                .map(|node| match node {
                    MergeSpineSeqNode::Zipped(node) => ISpineSeqNode::Zipped(self.convert(node)),
                    MergeSpineSeqNode::BothDeleted(left_del, right_del) => {
                        self.visit(&left_del.node);
                        self.visit(&right_del.node);
                        ISpineSeqNode::BothDeleted(left_del, right_del)
                    }
                    MergeSpineSeqNode::OneDeleteConflict(del, conflict_del, conflict_ins) => {
                        if self.can_merge(&del.node, &conflict_ins) {
                            let del = Colored {
                                node: self.merge(del.node, conflict_ins),
                                colors: del.colors,
                            };
                            self.visit(&conflict_del.node);
                            ISpineSeqNode::BothDeleted(del, conflict_del)
                        } else {
                            self.visit(&del.node);
                            self.visit(&conflict_del.node);
                            ISpineSeqNode::DeleteConflict(del, conflict_del, conflict_ins)
                        }
                    }
                    MergeSpineSeqNode::BothDeleteConflict(
                        left_del,
                        left_ins,
                        right_del,
                        right_ins,
                    ) => {
                        match (
                            self.can_merge(&right_del.node, &left_ins),
                            self.can_merge(&left_del.node, &right_ins),
                        ) {
                            (false, false) => {
                                self.visit(&left_del.node);
                                self.visit(&right_del.node);
                                ISpineSeqNode::DeleteConflict(
                                    left_del,
                                    right_del,
                                    self.merge(left_ins, right_ins),
                                )
                            }
                            (true, false) => {
                                let right_del = Colored {
                                    node: self.merge(right_del.node, left_ins),
                                    colors: right_del.colors,
                                };
                                self.visit(&left_del.node);
                                ISpineSeqNode::DeleteConflict(right_del, left_del, right_ins)
                            }
                            (false, true) => {
                                let left_del = Colored {
                                    node: self.merge(left_del.node, right_ins),
                                    colors: left_del.colors,
                                };
                                self.visit(&right_del.node);
                                ISpineSeqNode::DeleteConflict(left_del, right_del, left_ins)
                            }
                            (true, true) => {
                                // Solve both conflicts at once!
                                let left_del = Colored {
                                    node: self.merge(left_del.node, right_ins),
                                    colors: left_del.colors,
                                };
                                let right_del = Colored {
                                    node: self.merge(right_del.node, left_ins),
                                    colors: right_del.colors,
                                };
                                ISpineSeqNode::BothDeleted(left_del, right_del)
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
