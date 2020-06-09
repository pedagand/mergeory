use super::{Colored, DelNode, InsNode, InsSeq, InsSeqNode};
use crate::family_traits::Merge;

/// Merge insertion & deletion trees if they are identical modulo colors
pub struct IdMerger;

impl<I> Merge<InsNode<I>, InsNode<I>, InsNode<I>> for IdMerger
where
    IdMerger: Merge<Colored<I>, Colored<I>, Colored<I>>,
{
    fn can_merge(&mut self, left: &InsNode<I>, right: &InsNode<I>) -> bool {
        match (left, right) {
            (InsNode::InPlace(left), InsNode::InPlace(right)) => self.can_merge(left, right),
            (InsNode::Ellided(left), InsNode::Ellided(right)) => left.node == right.node,
            (InsNode::Conflict(left), InsNode::Conflict(right)) => {
                <IdMerger as Merge<Vec<InsNode<I>>, _, _>>::can_merge(self, left, right)
            }
            _ => false,
        }
    }

    fn merge(&mut self, left: InsNode<I>, right: InsNode<I>) -> InsNode<I> {
        match (left, right) {
            (InsNode::InPlace(left), InsNode::InPlace(right)) => {
                InsNode::InPlace(self.merge(left, right))
            }
            (InsNode::Ellided(left), InsNode::Ellided(right)) => InsNode::Ellided(Colored {
                node: left.node,
                colors: left.colors | right.colors,
            }),
            (InsNode::Conflict(left), InsNode::Conflict(right)) => InsNode::Conflict(
                <IdMerger as Merge<Vec<InsNode<I>>, _, _>>::merge(self, left, right),
            ),
            _ => panic!("IdMerger called on conflicting insertions"),
        }
    }
}

impl<I> Merge<InsSeq<I>, InsSeq<I>, InsSeq<I>> for IdMerger
where
    IdMerger: Merge<InsNode<I>, InsNode<I>, InsNode<I>>,
    IdMerger: Merge<
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
                    _ => panic!("IdMerger called on conflicting insertions"),
                })
                .collect(),
        )
    }
}

impl<D, I> Merge<DelNode<D, I>, InsNode<I>, DelNode<D, I>> for IdMerger
where
    IdMerger: Merge<D, I, D>,
{
    fn can_merge(&mut self, del: &DelNode<D, I>, ins: &InsNode<I>) -> bool {
        match (del, ins) {
            (DelNode::InPlace(del), InsNode::InPlace(ins)) => self.can_merge(del, &ins.node),
            (DelNode::Ellided(del_mv), InsNode::Ellided(ins_mv)) => *del_mv == ins_mv.node,
            (DelNode::MetavariableConflict(_, del, _), ins) => {
                <IdMerger as Merge<DelNode<D, I>, _, _>>::can_merge(self, del, ins)
            }
            _ => false,
        }
    }

    fn merge(&mut self, del: DelNode<D, I>, _: InsNode<I>) -> DelNode<D, I> {
        del
    }
}

impl<D, I> Merge<Vec<DelNode<D, I>>, InsSeq<I>, Vec<DelNode<D, I>>> for IdMerger
where
    IdMerger: Merge<DelNode<D, I>, InsNode<I>, DelNode<D, I>>,
{
    fn can_merge(&mut self, del_seq: &Vec<DelNode<D, I>>, ins_seq: &InsSeq<I>) -> bool {
        if del_seq.len() != ins_seq.0.len() {
            return false;
        }
        del_seq.iter().zip(&ins_seq.0).all(|(del, ins)| match ins {
            InsSeqNode::Node(ins) => self.can_merge(del, ins),
            _ => false,
        })
    }

    fn merge(&mut self, del: Vec<DelNode<D, I>>, _: InsSeq<I>) -> Vec<DelNode<D, I>> {
        del
    }
}
