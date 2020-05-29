use super::{Colored, DelNode, InsNode, InsSeq, InsSeqNode, SpineNode, SpineSeq, SpineSeqNode};
use crate::diff_tree::Metavariable;
use crate::family_traits::VisitMut;

pub struct MetavarRenamer<'a> {
    pub new_metavars: &'a mut Vec<Option<Metavariable>>,
    pub next_metavar: &'a mut usize,
}

impl<'a> VisitMut<Metavariable> for MetavarRenamer<'a> {
    fn visit_mut(&mut self, metavar: &mut Metavariable) {
        let next_metavar = &mut *self.next_metavar;
        if self.new_metavars.len() <= metavar.0 {
            self.new_metavars
                .resize_with(metavar.0 + 1, Default::default)
        }
        let repl_metavar = self.new_metavars[metavar.0].get_or_insert_with(|| {
            let mv_id = *next_metavar;
            *next_metavar += 1;
            Metavariable(mv_id)
        });
        *metavar = *repl_metavar
    }
}

impl<'a, T> VisitMut<Colored<T>> for MetavarRenamer<'a>
where
    MetavarRenamer<'a>: VisitMut<T>,
{
    fn visit_mut(&mut self, node: &mut Colored<T>) {
        self.visit_mut(&mut node.node)
    }
}

impl<'a, D, I> VisitMut<DelNode<D, I>> for MetavarRenamer<'a>
where
    MetavarRenamer<'a>: VisitMut<D>,
    MetavarRenamer<'a>: VisitMut<InsNode<I>>,
{
    fn visit_mut(&mut self, node: &mut DelNode<D, I>) {
        match node {
            DelNode::InPlace(subnode) => self.visit_mut(subnode),
            DelNode::Ellided(metavar) => self.visit_mut(metavar),
            DelNode::MetavariableConflict(metavar, del, ins) => {
                self.visit_mut(metavar);
                <MetavarRenamer as VisitMut<DelNode<D, I>>>::visit_mut(self, del);
                self.visit_mut(ins);
            }
        }
    }
}

impl<'a, I> VisitMut<InsNode<I>> for MetavarRenamer<'a>
where
    MetavarRenamer<'a>: VisitMut<I>,
    MetavarRenamer<'a>: VisitMut<Metavariable>,
{
    fn visit_mut(&mut self, node: &mut InsNode<I>) {
        match node {
            InsNode::InPlace(subnode) => self.visit_mut(subnode),
            InsNode::Ellided(metavar) => self.visit_mut(metavar),
            InsNode::Conflict(subnodes) => {
                for subnode in subnodes {
                    self.visit_mut(subnode)
                }
            }
        }
    }
}

impl<'a, I> VisitMut<InsSeqNode<I>> for MetavarRenamer<'a>
where
    MetavarRenamer<'a>: VisitMut<InsNode<I>>,
{
    fn visit_mut(&mut self, node: &mut InsSeqNode<I>) {
        match node {
            InsSeqNode::Node(node) => self.visit_mut(node),
            InsSeqNode::DeleteConflict(node) => self.visit_mut(node),
            InsSeqNode::InsertOrderConflict(conflicts) => {
                for ins_list in conflicts {
                    for ins in &mut ins_list.node {
                        self.visit_mut(ins)
                    }
                }
            }
        }
    }
}

impl<'a, I> VisitMut<InsSeq<I>> for MetavarRenamer<'a>
where
    MetavarRenamer<'a>: VisitMut<Vec<InsSeqNode<I>>>,
{
    fn visit_mut(&mut self, node: &mut InsSeq<I>) {
        self.visit_mut(&mut node.0)
    }
}

impl<'a, S, D, I> VisitMut<SpineNode<S, D, I>> for MetavarRenamer<'a>
where
    MetavarRenamer<'a>: VisitMut<S>,
    MetavarRenamer<'a>: VisitMut<DelNode<D, I>>,
    MetavarRenamer<'a>: VisitMut<InsNode<I>>,
{
    fn visit_mut(&mut self, node: &mut SpineNode<S, D, I>) {
        match node {
            SpineNode::Spine(spine) => self.visit_mut(spine),
            SpineNode::Unchanged => (),
            SpineNode::Changed(del, ins) => {
                self.visit_mut(del);
                self.visit_mut(ins);
            }
        }
    }
}

impl<'a, S, D, I> VisitMut<SpineSeq<S, D, I>> for MetavarRenamer<'a>
where
    MetavarRenamer<'a>: VisitMut<SpineNode<S, D, I>>,
    MetavarRenamer<'a>: VisitMut<DelNode<D, I>>,
    MetavarRenamer<'a>: VisitMut<InsNode<I>>,
{
    fn visit_mut(&mut self, seq: &mut SpineSeq<S, D, I>) {
        for node in &mut seq.0 {
            match node {
                SpineSeqNode::Zipped(spine) => self.visit_mut(spine),
                SpineSeqNode::Deleted(del) => self.visit_mut(&mut del.node),
                SpineSeqNode::DeleteConflict(del, ins) => {
                    self.visit_mut(&mut del.node);
                    self.visit_mut(ins);
                }
                SpineSeqNode::Inserted(ins) => self.visit_mut(&mut ins.node),
                SpineSeqNode::InsertOrderConflict(ins_vec) => {
                    for ins_seq in ins_vec {
                        for ins in &mut ins_seq.node {
                            self.visit_mut(ins)
                        }
                    }
                }
            }
        }
    }
}
