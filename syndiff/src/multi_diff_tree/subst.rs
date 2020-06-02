use super::merge_ins::MetavarStatus;
use super::{
    ColorSet, Colored, DelNode, InsNode, InsSeq, InsSeqNode, SpineNode, SpineSeq, SpineSeqNode,
};
use crate::ast;
use crate::diff_tree::Metavariable;
use crate::family_traits::{Convert, VisitMut};
use std::any::Any;

enum ComputedValue<U> {
    Ready(Box<dyn Any>),
    Unprocessed(U),
    Processing,
}

pub struct Substituter {
    del_subst: Vec<ComputedValue<Option<Box<dyn Any>>>>,
    ins_subst: Vec<ComputedValue<MetavarStatus>>,
}

impl Substituter {
    pub fn new(del_subst: Vec<Option<Box<dyn Any>>>, ins_subst: Vec<MetavarStatus>) -> Substituter {
        Substituter {
            del_subst: del_subst
                .into_iter()
                .map(ComputedValue::Unprocessed)
                .collect(),
            ins_subst: ins_subst
                .into_iter()
                .map(ComputedValue::Unprocessed)
                .collect(),
        }
    }
}

impl Substituter {
    fn del_subst<D, I>(&mut self, mv: Metavariable) -> DelNode<D, I>
    where
        Substituter: VisitMut<DelNode<D, I>>,
        DelNode<D, I>: Clone + 'static,
    {
        let repl = match std::mem::replace(&mut self.del_subst[mv.0], ComputedValue::Processing) {
            ComputedValue::Ready(repl_del) => *repl_del.downcast().unwrap(),
            ComputedValue::Unprocessed(None) => DelNode::Ellided(mv),
            ComputedValue::Unprocessed(Some(repl_del)) => {
                let mut repl_del = *repl_del.downcast().unwrap();
                self.visit_mut(&mut repl_del);
                repl_del
            }
            ComputedValue::Processing => panic!("Cycle in metavariable substitutions"),
        };
        self.del_subst[mv.0] = ComputedValue::Ready(Box::new(repl.clone()));
        repl
    }

    fn ins_subst<I: InsFromDel>(&mut self, mv: Colored<Metavariable>) -> InsNode<I>
    where
        Substituter: VisitMut<InsNode<I>>,
        Substituter: VisitMut<DelNode<I::Del, I>>,
        InferInsFromDel: Convert<DelNode<I::Del, I>, InsNode<I>>,
        InsNode<I>: Clone + 'static,
        DelNode<I::Del, I>: Clone + 'static,
    {
        let mv_id = mv.node.0;
        match std::mem::replace(&mut self.ins_subst[mv_id], ComputedValue::Processing) {
            ComputedValue::Ready(repl_ins) => {
                let subst = *repl_ins.downcast::<InsNode<I>>().unwrap();
                self.ins_subst[mv_id] = ComputedValue::Ready(Box::new(subst.clone()));
                subst
            }
            ComputedValue::Unprocessed(MetavarStatus::Keep) => {
                // Keep unprocessed here to allow different color sets
                self.ins_subst[mv_id] = ComputedValue::Unprocessed(MetavarStatus::Keep);
                // Build the insertion substitution from the deletion substitution
                let del_subst = self.del_subst(mv.node);
                InferInsFromDel(mv.colors).convert(del_subst)
            }
            ComputedValue::Unprocessed(MetavarStatus::Replace(repl_ins)) => {
                let mut repl_ins = *repl_ins.downcast::<InsNode<I>>().unwrap();
                self.visit_mut(&mut repl_ins);
                self.ins_subst[mv_id] = ComputedValue::Ready(Box::new(repl_ins.clone()));
                repl_ins
            }
            ComputedValue::Unprocessed(MetavarStatus::Conflict) => {
                // Keep unprocessed here to allow different color sets
                self.ins_subst[mv_id] = ComputedValue::Unprocessed(MetavarStatus::Conflict);
                InsNode::Ellided(mv)
            }
            ComputedValue::Processing => panic!("Cycle in metavariable substitutions"),
        }
    }
}

impl<D, I> VisitMut<DelNode<D, I>> for Substituter
where
    Substituter: VisitMut<D>,
    Substituter: VisitMut<InsNode<I>>,
    DelNode<D, I>: Clone + 'static,
{
    fn visit_mut(&mut self, node: &mut DelNode<D, I>) {
        match node {
            DelNode::InPlace(del) => self.visit_mut(del),
            DelNode::Ellided(mv) => *node = self.del_subst(*mv),
            DelNode::MetavariableConflict(_, del, ins) => {
                <Substituter as VisitMut<DelNode<D, I>>>::visit_mut(self, del);
                self.visit_mut(ins);
            }
        }
    }
}

impl<I: InsFromDel> VisitMut<InsNode<I>> for Substituter
where
    Substituter: VisitMut<I>,
    Substituter: VisitMut<I::Del>,
    InferInsFromDel: Convert<I::Del, I>,
    InsNode<I>: Clone + 'static,
    DelNode<I::Del, I>: Clone + 'static,
{
    fn visit_mut(&mut self, node: &mut InsNode<I>) {
        match node {
            InsNode::InPlace(ins) => self.visit_mut(&mut ins.node),
            InsNode::Ellided(mv) => *node = self.ins_subst(*mv),
            InsNode::Conflict(ins_list) => {
                for ins in ins_list {
                    <Substituter as VisitMut<InsNode<I>>>::visit_mut(self, ins)
                }
            }
        }
    }
}

impl<I> VisitMut<InsSeq<I>> for Substituter
where
    Substituter: VisitMut<InsNode<I>>,
{
    fn visit_mut(&mut self, seq: &mut InsSeq<I>) {
        for node in &mut seq.0 {
            match node {
                InsSeqNode::Node(node) => self.visit_mut(node),
                InsSeqNode::DeleteConflict(node) => self.visit_mut(node),
                InsSeqNode::InsertOrderConflict(ins_vec) => {
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

impl<S, D, I> VisitMut<SpineNode<S, D, I>> for Substituter
where
    Substituter: VisitMut<S>,
    Substituter: VisitMut<DelNode<D, I>>,
    Substituter: VisitMut<InsNode<I>>,
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

impl<S, D, I> VisitMut<SpineSeq<S, D, I>> for Substituter
where
    Substituter: VisitMut<SpineNode<S, D, I>>,
    Substituter: VisitMut<DelNode<D, I>>,
    Substituter: VisitMut<InsNode<I>>,
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

pub struct InferInsFromDel(ColorSet);

impl<D, I> Convert<DelNode<D, I>, InsNode<I>> for InferInsFromDel
where
    InferInsFromDel: Convert<D, I>,
{
    fn convert(&mut self, del: DelNode<D, I>) -> InsNode<I> {
        match del {
            DelNode::InPlace(del) => InsNode::InPlace(Colored {
                node: self.convert(del),
                colors: self.0,
            }),
            DelNode::Ellided(mv) | DelNode::MetavariableConflict(mv, _, _) => {
                InsNode::Ellided(Colored {
                    node: mv,
                    colors: self.0,
                })
            }
        }
    }
}

impl<D, I> Convert<Vec<DelNode<D, I>>, InsSeq<I>> for InferInsFromDel
where
    InferInsFromDel: Convert<DelNode<D, I>, InsNode<I>>,
{
    fn convert(&mut self, del_seq: Vec<DelNode<D, I>>) -> InsSeq<I> {
        InsSeq(
            del_seq
                .into_iter()
                .map(|node| InsSeqNode::Node(self.convert(node)))
                .collect(),
        )
    }
}

pub trait InsFromDel {
    type Del;
}

macro_rules! impl_ins_from_del {
    { $($ast_typ:ident),* } => {
        $(impl InsFromDel for ast::multi_diff::ins::$ast_typ {
            type Del = ast::multi_diff::del::$ast_typ;
        })*
    }
}
impl_ins_from_del!(Expr, Stmt, Item, TraitItem, ImplItem, ForeignItem);
