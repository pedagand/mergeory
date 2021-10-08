use super::id_merger::IdMerger;
use super::{
    ColorSet, Colored, DelNode, InsNode, InsSeq, InsSeqNode, SpineNode, SpineSeq, SpineSeqNode,
};
use crate::ast::multi_diff::DelEquivType;
use crate::diff_tree::Metavariable;
use crate::family_traits::{Convert, Merge, VisitMut};
use std::any::Any;

enum ComputableSubst<T, U> {
    Pending(U),
    Processing,
    Computed(T),
}

pub struct Substituter {
    del_subst: Vec<ComputableSubst<Box<dyn Any>, Option<Box<dyn Any>>>>,
    ins_subst: Vec<ComputableSubst<Option<Box<dyn Any>>, Vec<Option<Box<dyn Any>>>>>,
    ins_cycle_stack: Vec<(Metavariable, bool)>,
}

impl Substituter {
    pub fn new(
        del_subst: Vec<Option<Box<dyn Any>>>,
        ins_subst: Vec<Vec<Option<Box<dyn Any>>>>,
    ) -> Substituter {
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

    // Warning: The colors on the returned tree are arbitrary and should be replaced or discarded
    fn del_subst<D, I>(&mut self, mv: Metavariable) -> DelNode<D, I>
    where
        Substituter: VisitMut<DelNode<D, I>>,
        DelNode<D, I>: Clone + 'static,
    {
        let repl = match std::mem::replace(&mut self.del_subst[mv.0], ComputableSubst::Processing) {
            ComputableSubst::Computed(repl_del) => *repl_del.downcast().unwrap(),
            ComputableSubst::Pending(None) => DelNode::Elided(Colored::new_white(mv)),
            ComputableSubst::Pending(Some(repl_del)) => {
                let mut repl_del = *repl_del.downcast().unwrap();
                self.visit_mut(&mut repl_del);
                repl_del
            }
            ComputableSubst::Processing => {
                // In del_subst cycles can only occur between metavariables that should be all
                // unified together. Break the cycle by behaving once as identity.
                DelNode::Elided(Colored::new_white(mv))
            }
        };
        self.del_subst[mv.0] = ComputableSubst::Computed(Box::new(repl.clone()));
        repl
    }

    fn ins_subst<I: DelEquivType>(&mut self, mv: Metavariable) -> InsNode<I>
    where
        Substituter: VisitMut<InsNode<I>>,
        Substituter: VisitMut<DelNode<I::DelEquivType, I>>,
        IdMerger: Merge<InsNode<I>, InsNode<I>, InsNode<I>>,
        InferInsFromDel: Convert<DelNode<I::DelEquivType, I>, InsNode<I>>,
        InsNode<I>: Clone + 'static,
        DelNode<I::DelEquivType, I>: Clone + 'static,
    {
        let subst = match std::mem::replace(&mut self.ins_subst[mv.0], ComputableSubst::Processing)
        {
            ComputableSubst::Computed(subst) => subst.map(|x| *x.downcast::<InsNode<I>>().unwrap()),
            ComputableSubst::Pending(replacements) => {
                self.ins_cycle_stack.push((mv, false));
                let mut repl_ins = replacements
                    .into_iter()
                    .map(|repl| match repl {
                        None => {
                            // Build the insertion substitution from the deletion substitution
                            InferInsFromDel.convert(self.del_subst(mv))
                        }
                        Some(ins) => {
                            let mut ins = *ins.downcast::<InsNode<I>>().unwrap();
                            self.visit_mut(&mut ins);
                            ins
                        }
                    })
                    .collect::<Vec<_>>();
                let (cycle_mv, cycle) = self.ins_cycle_stack.pop().unwrap();
                assert!(cycle_mv == mv);
                if !cycle {
                    // No cycle during computation on potential replacements, try to fuse them
                    let last_ins = repl_ins.pop().unwrap();
                    repl_ins.into_iter().try_fold(last_ins, |acc, ins| {
                        if IdMerger.can_merge(&acc, &ins) {
                            Some(IdMerger.merge(acc, ins))
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            }
            ComputableSubst::Processing => {
                // If a cycle occur in ins_subst, we should yield a conflict for all metavariables
                // in that cycle.
                for (stack_mv, cycle_flag) in self.ins_cycle_stack.iter_mut().rev() {
                    *cycle_flag = true;
                    if *stack_mv == mv {
                        break;
                    }
                }
                assert!(!self.ins_cycle_stack[0].1 || self.ins_cycle_stack[0].0 == mv);
                None
            }
        };
        match subst {
            Some(subst) => {
                self.ins_subst[mv.0] = ComputableSubst::Computed(Some(Box::new(subst.clone())));
                subst
            }
            None => {
                // Save conflict and return a simple white metavariable
                self.ins_subst[mv.0] = ComputableSubst::Computed(None);
                InsNode::Elided(mv)
            }
        }
    }
}

impl<D, I> VisitMut<DelNode<D, I>> for Substituter
where
    Substituter: VisitMut<D>,
    DelNode<D, I>: Clone + 'static,
    ColorReplacer: VisitMut<DelNode<D, I>>,
{
    fn visit_mut(&mut self, node: &mut DelNode<D, I>) {
        match node {
            DelNode::InPlace(del) => self.visit_mut(&mut del.node),
            DelNode::Elided(mv) => {
                let mut subst = self.del_subst(mv.node);
                ColorReplacer(mv.colors).visit_mut(&mut subst);
                *node = subst;
            }
            DelNode::MetavariableConflict(_, del, _) => {
                // The insertion part will be visited only if the conflict stays
                <Substituter as VisitMut<DelNode<D, I>>>::visit_mut(self, del)
            }
        }
    }
}

impl<I: DelEquivType> VisitMut<InsNode<I>> for Substituter
where
    Substituter: VisitMut<I>,
    Substituter: VisitMut<DelNode<I::DelEquivType, I>>,
    IdMerger: Merge<InsNode<I>, InsNode<I>, InsNode<I>>,
    InferInsFromDel: Convert<I::DelEquivType, I>,
    InsNode<I>: Clone + 'static,
    DelNode<I::DelEquivType, I>: Clone + 'static,
{
    fn visit_mut(&mut self, node: &mut InsNode<I>) {
        match node {
            InsNode::InPlace(ins) => self.visit_mut(&mut ins.node),
            InsNode::Elided(mv) => *node = self.ins_subst(*mv),
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
                SpineSeqNode::Deleted(del) => self.visit_mut(del),
                SpineSeqNode::DeleteConflict(del, ins) => {
                    self.visit_mut(del);
                    self.visit_mut(ins);

                    // Solve the delete conflict if del and ins are identical after substitution
                    // ARGH! I don't understand why I need manual type annotation here...
                    if Merge::<DelNode<D, I>, _, _>::can_merge(&mut IdMerger, del, ins) {
                        *node = SpineSeqNode::Deleted(std::mem::replace(
                            del,
                            DelNode::Elided(Colored::new_white(Metavariable(usize::MAX))),
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

pub struct ColorReplacer(ColorSet);

impl<D, I> VisitMut<DelNode<D, I>> for ColorReplacer
where
    ColorReplacer: VisitMut<D>,
{
    fn visit_mut(&mut self, node: &mut DelNode<D, I>) {
        match node {
            DelNode::InPlace(del) => {
                self.visit_mut(&mut del.node);
                del.colors = self.0;
            }
            DelNode::Elided(mv) => mv.colors = self.0,
            DelNode::MetavariableConflict(_, del, _) => {
                VisitMut::<DelNode<D, I>>::visit_mut(self, del)
            }
        }
    }
}

pub struct InferInsFromDel;

impl<D, I> Convert<DelNode<D, I>, InsNode<I>> for InferInsFromDel
where
    InferInsFromDel: Convert<D, I>,
{
    fn convert(&mut self, del: DelNode<D, I>) -> InsNode<I> {
        match del {
            DelNode::InPlace(del) => InsNode::InPlace(Colored::new_white(self.convert(del.node))),
            DelNode::Elided(mv) => InsNode::Elided(mv.node),
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

pub struct SolvedConflictsRemover(pub Substituter);

impl<D, I> VisitMut<DelNode<D, I>> for SolvedConflictsRemover
where
    SolvedConflictsRemover: VisitMut<D>,
    Substituter: VisitMut<InsNode<I>>,
{
    fn visit_mut(&mut self, node: &mut DelNode<D, I>) {
        match node {
            DelNode::InPlace(del) => self.visit_mut(&mut del.node),
            DelNode::Elided(_) => (),
            DelNode::MetavariableConflict(mv, del, repl) => {
                VisitMut::<DelNode<D, I>>::visit_mut(self, del);
                match &self.0.ins_subst[mv.0] {
                    ComputableSubst::Computed(Some(_)) => {
                        *node =
                            std::mem::replace(&mut **del, DelNode::Elided(Colored::new_white(*mv)))
                    }
                    ComputableSubst::Computed(None) => match repl {
                        None => (),
                        Some(ins) => self.0.visit_mut(ins),
                    },
                    ComputableSubst::Pending(_) => {
                        match repl {
                            None => {
                                *node = std::mem::replace(
                                    &mut **del,
                                    DelNode::Elided(Colored::new_white(*mv)),
                                )
                            }
                            Some(ins) => {
                                // We are dealing here with a subtree inlined into a metavariable
                                // that was never inserted back.
                                // Removing the conflict would arbitrarily drop a modification so
                                // we keep the metavariable conflict.
                                // We might accidentally call this too late and consider unused a
                                // metavariable replacement used inside another metavariable
                                // conflict, but all other solutions seem worse.
                                self.0.visit_mut(ins)
                            }
                        }
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
                SpineSeqNode::Deleted(del) => self.visit_mut(del),
                SpineSeqNode::DeleteConflict(del, _) => self.visit_mut(del),
                SpineSeqNode::Inserted(_) | SpineSeqNode::InsertOrderConflict(_) => (),
            }
        }
    }
}
