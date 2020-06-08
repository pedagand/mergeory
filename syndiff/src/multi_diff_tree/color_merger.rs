use super::{Colored, InsNode, InsSeq, InsSeqNode};
use crate::family_traits::Merge;

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
            (InsNode::Ellided(left), InsNode::Ellided(right)) => InsNode::Ellided(Colored {
                node: left.node,
                colors: left.colors | right.colors,
            }),
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
