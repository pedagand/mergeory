use super::{DelNode, InsNode, InsSeq, InsSeqNode, SpineNode, SpineSeq, SpineSeqNode};
use crate::family_traits::Visit;

pub struct ConflictCounter(u64);

impl<D, I> Visit<DelNode<D, I>> for ConflictCounter
where
    ConflictCounter: Visit<D>,
    ConflictCounter: Visit<I>,
{
    fn visit(&mut self, node: &DelNode<D, I>) {
        match node {
            DelNode::InPlace(del) => self.visit(&del.node),
            DelNode::Elided(_) => (),
            DelNode::MetavariableConflict(_, del, repl) => {
                self.0 += 1;
                self.visit(del);
                match repl {
                    None => (),
                    Some(ins) => self.visit(ins),
                }
            }
        }
    }
}

impl<I> Visit<InsNode<I>> for ConflictCounter
where
    ConflictCounter: Visit<I>,
{
    fn visit(&mut self, node: &InsNode<I>) {
        match node {
            InsNode::InPlace(ins) => self.visit(&ins.node),
            InsNode::Elided(_) => (),
            InsNode::Conflict(ins_list) => {
                self.0 += 1;
                for ins in ins_list {
                    <ConflictCounter as Visit<InsNode<I>>>::visit(self, ins)
                }
            }
        }
    }
}

impl<I> Visit<InsSeq<I>> for ConflictCounter
where
    ConflictCounter: Visit<InsNode<I>>,
{
    fn visit(&mut self, seq: &InsSeq<I>) {
        for node in &seq.0 {
            match node {
                InsSeqNode::Node(ins) => self.visit(ins),
                InsSeqNode::DeleteConflict(ins) => {
                    self.0 += 1;
                    self.visit(ins);
                }
                InsSeqNode::InsertOrderConflict(ins_list) => {
                    self.0 += 1;
                    for ins_set in ins_list {
                        for ins in &ins_set.node {
                            self.visit(ins)
                        }
                    }
                }
            }
        }
    }
}

impl<S, D, I> Visit<SpineNode<S, D, I>> for ConflictCounter
where
    ConflictCounter: Visit<S>,
    ConflictCounter: Visit<DelNode<D, I>>,
    ConflictCounter: Visit<InsNode<I>>,
{
    fn visit(&mut self, node: &SpineNode<S, D, I>) {
        match node {
            SpineNode::Spine(spine) => self.visit(spine),
            SpineNode::Unchanged => (),
            SpineNode::Changed(del, ins) => {
                self.visit(del);
                self.visit(ins);
            }
        }
    }
}

impl<S, D, I> Visit<SpineSeq<S, D, I>> for ConflictCounter
where
    ConflictCounter: Visit<SpineNode<S, D, I>>,
    ConflictCounter: Visit<DelNode<D, I>>,
    ConflictCounter: Visit<InsNode<I>>,
{
    fn visit(&mut self, seq: &SpineSeq<S, D, I>) {
        for node in &seq.0 {
            match node {
                SpineSeqNode::Zipped(spine) => self.visit(spine),
                SpineSeqNode::Deleted(del) => self.visit(del),
                SpineSeqNode::DeleteConflict(del, ins) => {
                    self.0 += 1;
                    self.visit(del);
                    self.visit(ins);
                }
                SpineSeqNode::Inserted(ins_list) => {
                    for ins in &ins_list.node {
                        self.visit(ins);
                    }
                }
                SpineSeqNode::InsertOrderConflict(ins_list) => {
                    self.0 += 1;
                    for ins_set in ins_list {
                        for ins in &ins_set.node {
                            self.visit(ins)
                        }
                    }
                }
            }
        }
    }
}

pub fn count_conflicts<T>(multi_diff: &T) -> u64
where
    ConflictCounter: Visit<T>,
{
    let mut counter = ConflictCounter(0);
    counter.visit(multi_diff);
    counter.0
}
