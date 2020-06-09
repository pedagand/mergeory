use super::id_merger::IdMerger;
use super::merge_ins::MetavarStatus;
use super::{
    ColorSet, Colored, DelNode, InsNode, InsSeq, InsSeqNode, SpineNode, SpineSeq, SpineSeqNode,
};
use crate::ast;
use crate::diff_tree::Metavariable;
use crate::family_traits::{Convert, Merge, VisitMut};
use std::any::Any;

enum ComputableSubst<U> {
    Pending(U),
    Processing,
    Computed(Box<dyn Any>),
}

pub struct Substituter {
    del_subst: Vec<ComputableSubst<Option<Box<dyn Any>>>>,
    ins_subst: Vec<ComputableSubst<MetavarStatus>>,
    ins_cycle_stack: Vec<(Metavariable, bool)>,
}

impl Substituter {
    pub fn new(del_subst: Vec<Option<Box<dyn Any>>>, ins_subst: Vec<MetavarStatus>) -> Substituter {
        Substituter {
            del_subst: del_subst
                .into_iter()
                .map(ComputableSubst::Pending)
                .collect(),
            ins_subst: ins_subst
                .into_iter()
                .map(ComputableSubst::Pending)
                .collect(),
            ins_cycle_stack: Vec::new(),
        }
    }

    fn del_subst<D, I>(&mut self, mv: Metavariable) -> DelNode<D, I>
    where
        Substituter: VisitMut<DelNode<D, I>>,
        DelNode<D, I>: Clone + 'static,
    {
        let repl = match std::mem::replace(&mut self.del_subst[mv.0], ComputableSubst::Processing) {
            ComputableSubst::Computed(repl_del) => *repl_del.downcast().unwrap(),
            ComputableSubst::Pending(None) => DelNode::Ellided(mv),
            ComputableSubst::Pending(Some(repl_del)) => {
                let mut repl_del = *repl_del.downcast().unwrap();
                self.visit_mut(&mut repl_del);
                repl_del
            }
            ComputableSubst::Processing => {
                // In del_subst cycles can only occur between metavariables that should be all
                // unified together. Break the cycle by behaving once as identity.
                DelNode::Ellided(mv)
            }
        };
        self.del_subst[mv.0] = ComputableSubst::Computed(Box::new(repl.clone()));
        repl
    }

    fn ins_subst<I: InsFromDel>(&mut self, mv: Colored<Metavariable>) -> InsNode<I>
    where
        Substituter: VisitMut<InsNode<I>>,
        Substituter: VisitMut<DelNode<I::Del, I>>,
        IdMerger: Merge<InsNode<I>, InsNode<I>, InsNode<I>>,
        InferInsFromDel: Convert<DelNode<I::Del, I>, InsNode<I>>,
        InsNode<I>: Clone + 'static,
        DelNode<I::Del, I>: Clone + 'static,
    {
        let mv_id = mv.node.0;
        let subst = match std::mem::replace(&mut self.ins_subst[mv_id], ComputableSubst::Processing)
        {
            ComputableSubst::Computed(repl_ins) => {
                let subst = *repl_ins.downcast::<InsNode<I>>().unwrap();
                self.ins_subst[mv_id] = ComputableSubst::Computed(Box::new(subst.clone()));
                Some(subst)
            }
            ComputableSubst::Pending(MetavarStatus::Keep) => {
                // Recompute everytime to allow different color sets
                self.ins_subst[mv_id] = ComputableSubst::Pending(MetavarStatus::Keep);
                // Build the insertion substitution from the deletion substitution
                let del_subst = self.del_subst(mv.node);
                Some(InferInsFromDel(mv.colors).convert(del_subst))
            }
            ComputableSubst::Pending(MetavarStatus::Replace(repl_ins)) => {
                self.ins_cycle_stack.push((mv.node, false));
                let mut repl_ins = *repl_ins.downcast::<Vec<InsNode<I>>>().unwrap();
                for ins in &mut repl_ins {
                    self.visit_mut(ins)
                }
                let (cycle_mv, cycle) = self.ins_cycle_stack.pop().unwrap();
                assert!(cycle_mv == mv.node);
                if !cycle {
                    // No cycle during computation on potential replacements, try to fuse them
                    let last_ins = repl_ins.pop().unwrap();
                    let merged_ins = repl_ins.into_iter().try_fold(last_ins, |acc, ins| {
                        if IdMerger.can_merge(&acc, &ins) {
                            Some(IdMerger.merge(acc, ins))
                        } else {
                            None
                        }
                    });
                    match merged_ins {
                        Some(repl_ins) => {
                            // All the potential substitutions are mergeable
                            self.ins_subst[mv_id] =
                                ComputableSubst::Computed(Box::new(repl_ins.clone()));
                            Some(repl_ins)
                        }
                        None => None,
                    }
                } else {
                    None
                }
            }
            ComputableSubst::Pending(MetavarStatus::Conflict) => None,
            ComputableSubst::Processing => {
                // If a cycle occur in ins_subst, we should yield a conflict for all metavariables
                // in that cycle.
                for (stack_mv, cycle_flag) in self.ins_cycle_stack.iter_mut().rev() {
                    *cycle_flag = true;
                    if *stack_mv == mv.node {
                        break;
                    }
                }
                assert!(!self.ins_cycle_stack[0].1 || self.ins_cycle_stack[0].0 == mv.node);
                None
            }
        };
        match subst {
            Some(subst) => subst,
            None => {
                // Save conflict and return a simple colored metavariable
                self.ins_subst[mv_id] = ComputableSubst::Pending(MetavarStatus::Conflict);
                InsNode::Ellided(mv)
            }
        }
    }
}

impl<D, I> VisitMut<DelNode<D, I>> for Substituter
where
    Substituter: VisitMut<D>,
    DelNode<D, I>: Clone + 'static,
{
    fn visit_mut(&mut self, node: &mut DelNode<D, I>) {
        match node {
            DelNode::InPlace(del) => self.visit_mut(del),
            DelNode::Ellided(mv) => *node = self.del_subst(*mv),
            DelNode::MetavariableConflict(_, del, _) => {
                // The insertion part will be visited only if the conflict stays
                <Substituter as VisitMut<DelNode<D, I>>>::visit_mut(self, del)
            }
        }
    }
}

impl<I: InsFromDel> VisitMut<InsNode<I>> for Substituter
where
    Substituter: VisitMut<I>,
    Substituter: VisitMut<I::Del>,
    IdMerger: Merge<InsNode<I>, InsNode<I>, InsNode<I>>,
    InferInsFromDel: Convert<I::Del, I>,
    InsNode<I>: Clone + 'static,
    DelNode<I::Del, I>: Clone + 'static,
{
    fn visit_mut(&mut self, node: &mut InsNode<I>) {
        match node {
            InsNode::InPlace(ins) => self.visit_mut(&mut ins.node),
            InsNode::Ellided(mv) => *node = self.ins_subst(*mv),
            InsNode::Conflict(conflict_list) => {
                for ins in &mut *conflict_list {
                    <Substituter as VisitMut<InsNode<I>>>::visit_mut(self, ins)
                }

                // Try to solve the insertion conflict after substitution
                let mut conflict_list_iter = std::mem::take(conflict_list).into_iter();
                let mut cur_ins = conflict_list_iter.next().unwrap();
                for ins in conflict_list_iter {
                    if IdMerger.can_merge(&cur_ins, &ins) {
                        cur_ins = IdMerger.merge(cur_ins, ins)
                    } else {
                        conflict_list.push(cur_ins);
                        cur_ins = ins
                    }
                }
                if conflict_list.is_empty() {
                    *node = cur_ins
                } else {
                    conflict_list.push(cur_ins);
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
                InsSeqNode::InsertOrderConflict(conflict_list) => {
                    for ins_seq in conflict_list {
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
    IdMerger: Merge<Colored<Vec<InsNode<I>>>, Colored<Vec<InsNode<I>>>, Colored<Vec<InsNode<I>>>>,
    IdMerger: Merge<DelNode<D, I>, InsNode<I>, DelNode<D, I>>,
{
    fn visit_mut(&mut self, seq: &mut SpineSeq<S, D, I>) {
        for node in &mut seq.0 {
            match node {
                SpineSeqNode::Zipped(spine) => self.visit_mut(spine),
                SpineSeqNode::Deleted(del) => self.visit_mut(&mut del.node),
                SpineSeqNode::DeleteConflict(del, ins) => {
                    self.visit_mut(&mut del.node);
                    self.visit_mut(ins);

                    // Solve the delete conflict if del and ins are identical after substitution
                    // ARGH! I don't understand why I need manual type annotation here...
                    if Merge::<DelNode<D, I>, _, _>::can_merge(&mut IdMerger, &del.node, ins) {
                        *node = SpineSeqNode::Deleted(std::mem::replace(
                            del,
                            Colored::new_white(DelNode::Ellided(Metavariable(usize::MAX))),
                        ))
                    }
                }
                SpineSeqNode::Inserted(ins_seq) => self.visit_mut(&mut ins_seq.node),
                SpineSeqNode::InsertOrderConflict(conflict_list) => {
                    for ins_seq in &mut *conflict_list {
                        for ins in &mut ins_seq.node {
                            self.visit_mut(ins)
                        }
                    }

                    // Try to solve the insert order conflict after substitutions
                    let mut conflict_list_iter = std::mem::take(conflict_list).into_iter();
                    let mut cur_ins_seq = conflict_list_iter.next().unwrap();
                    for ins_seq in conflict_list_iter {
                        if IdMerger.can_merge(&cur_ins_seq, &ins_seq) {
                            cur_ins_seq = IdMerger.merge(cur_ins_seq, ins_seq)
                        } else {
                            conflict_list.push(cur_ins_seq);
                            cur_ins_seq = ins_seq
                        }
                    }
                    if conflict_list.is_empty() {
                        *node = SpineSeqNode::Inserted(cur_ins_seq);
                    } else {
                        conflict_list.push(cur_ins_seq);
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
            DelNode::Ellided(mv) => InsNode::Ellided(Colored {
                node: mv,
                colors: self.0,
            }),
            DelNode::MetavariableConflict(_, del, _) => {
                Convert::<DelNode<D, I>, _>::convert(self, *del)
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

pub struct SolvedConflictsRemover(pub Substituter);

impl<D, I> VisitMut<DelNode<D, I>> for SolvedConflictsRemover
where
    SolvedConflictsRemover: VisitMut<D>,
    Substituter: VisitMut<InsNode<I>>,
{
    fn visit_mut(&mut self, node: &mut DelNode<D, I>) {
        match node {
            DelNode::InPlace(del) => self.visit_mut(del),
            DelNode::Ellided(_) => (),
            DelNode::MetavariableConflict(mv, del, ins) => {
                VisitMut::<DelNode<D, I>>::visit_mut(self, del);
                match &self.0.ins_subst[mv.0] {
                    ComputableSubst::Computed(_)
                    | ComputableSubst::Pending(MetavarStatus::Keep) => {
                        *node = std::mem::replace(&mut **del, DelNode::Ellided(*mv))
                    }
                    ComputableSubst::Pending(MetavarStatus::Conflict)
                    | ComputableSubst::Pending(MetavarStatus::Replace(_)) => {
                        // The Replace case is here for dealing with metavariables that are never
                        // inserted back. They should conflict to keep their insertion trees.
                        // We might accidentally call this too late and considered unused a
                        // metavariable replacement used inside another metavariable conflict, but
                        // all other solutions seem worse.
                        self.0.visit_mut(ins)
                    }
                    ComputableSubst::Processing => {
                        panic!("Still processing a metavariable while removing solved conflicts")
                    }
                }
            }
        }
    }
}

impl<S, D, I> VisitMut<SpineNode<S, D, I>> for SolvedConflictsRemover
where
    SolvedConflictsRemover: VisitMut<S>,
    SolvedConflictsRemover: VisitMut<DelNode<D, I>>,
{
    fn visit_mut(&mut self, node: &mut SpineNode<S, D, I>) {
        match node {
            SpineNode::Spine(spine) => self.visit_mut(spine),
            SpineNode::Unchanged => (),
            SpineNode::Changed(del, _) => self.visit_mut(del),
        }
    }
}

impl<S, D, I> VisitMut<SpineSeq<S, D, I>> for SolvedConflictsRemover
where
    SolvedConflictsRemover: VisitMut<SpineNode<S, D, I>>,
    SolvedConflictsRemover: VisitMut<DelNode<D, I>>,
{
    fn visit_mut(&mut self, seq: &mut SpineSeq<S, D, I>) {
        for node in &mut seq.0 {
            match node {
                SpineSeqNode::Zipped(spine) => self.visit_mut(spine),
                SpineSeqNode::Deleted(del) => self.visit_mut(&mut del.node),
                SpineSeqNode::DeleteConflict(del, _) => self.visit_mut(&mut del.node),
                SpineSeqNode::Inserted(_) | SpineSeqNode::InsertOrderConflict(_) => (),
            }
        }
    }
}
