use super::id_merger::IdMerger;
use super::{
    ColorSet, Colored, DelNode, InsNode, InsSeq, InsSeqNode, SpineNode, SpineSeq, SpineSeqNode,
};
use crate::family_traits::{Convert, Merge, VisitMut};
use std::any::Any;

pub struct MetavarRemover {
    metavar_replacements: Vec<Option<Box<dyn Any>>>,
    metavar_conflict: Vec<bool>,
    mergeable: bool,
}

impl<D, I, T> Merge<DelNode<D, I>, T, DelNode<D, I>> for MetavarRemover
where
    MetavarRemover: Merge<D, T, D>,
    InferFromSyn: Convert<T, InsNode<I>>,
    InferFromSynColored: Convert<T, DelNode<D, I>>,
    IdMerger: Merge<InsNode<I>, InsNode<I>, InsNode<I>>,
    InsNode<I>: 'static,
    T: Clone,
{
    fn can_merge(&mut self, diff: &DelNode<D, I>, source: &T) -> bool {
        // Here just compare the in place nodes, without caring about unification problems
        match diff {
            DelNode::InPlace(d) => self.can_merge(&d.node, source),
            DelNode::Elided(_) => true,
            DelNode::MetavariableConflict(_, d, _) => {
                Merge::<DelNode<D, I>, T, _>::can_merge(self, d, source)
            }
        }
    }

    fn merge(&mut self, diff: DelNode<D, I>, source: T) -> DelNode<D, I> {
        match diff {
            DelNode::InPlace(d) => DelNode::InPlace(Colored {
                node: self.merge(d.node, source),
                colors: d.colors,
            }),
            DelNode::Elided(mv) => {
                let mv_id = mv.node.0;
                if self.metavar_replacements.len() <= mv_id {
                    self.metavar_replacements
                        .resize_with(mv_id + 1, Default::default);
                    self.metavar_conflict.resize(mv_id + 1, false);
                }

                let ins_repl = InferFromSyn.convert(source.clone());
                match &self.metavar_replacements[mv_id] {
                    None => self.metavar_replacements[mv_id] = Some(Box::new(ins_repl)),
                    Some(cur_repl) => {
                        let cur_repl = cur_repl.downcast_ref::<InsNode<I>>().unwrap();
                        if !IdMerger.can_merge(cur_repl, &ins_repl) {
                            self.mergeable = false;
                        }
                    }
                }
                InferFromSynColored(mv.colors).convert(source)
            }
            DelNode::MetavariableConflict(mv, del, ins) => {
                if self.metavar_replacements.len() <= mv.0 {
                    self.metavar_replacements
                        .resize_with(mv.0 + 1, Default::default);
                    self.metavar_conflict.resize(mv.0 + 1, false);
                }
                self.metavar_conflict[mv.0] = true;
                DelNode::MetavariableConflict(
                    mv,
                    Box::new(Merge::<DelNode<D, I>, T, _>::merge(self, *del, source)),
                    ins,
                )
            }
        }
    }
}

impl<S, D, I, T> Merge<SpineNode<S, D, I>, T, SpineNode<S, D, I>> for MetavarRemover
where
    MetavarRemover: Merge<S, T, S>,
    MetavarRemover: Merge<DelNode<D, I>, T, DelNode<D, I>>,
    InferFromSyn: Convert<T, S>,
{
    fn can_merge(&mut self, diff: &SpineNode<S, D, I>, source: &T) -> bool {
        match diff {
            SpineNode::Spine(spine) => self.can_merge(spine, source),
            SpineNode::Unchanged => true,
            SpineNode::Changed(del, _) => self.can_merge(del, source),
        }
    }

    fn merge(&mut self, diff: SpineNode<S, D, I>, source: T) -> SpineNode<S, D, I> {
        match diff {
            SpineNode::Spine(spine) => SpineNode::Spine(self.merge(spine, source)),
            SpineNode::Unchanged => SpineNode::Spine(InferFromSyn.convert(source)),
            SpineNode::Changed(del, ins) => SpineNode::Changed(self.merge(del, source), ins),
        }
    }
}

impl<S, D, I, T> Merge<SpineSeq<S, D, I>, Vec<T>, SpineSeq<S, D, I>> for MetavarRemover
where
    MetavarRemover: Merge<SpineNode<S, D, I>, T, SpineNode<S, D, I>>,
    MetavarRemover: Merge<DelNode<D, I>, T, DelNode<D, I>>,
{
    fn can_merge(&mut self, diff: &SpineSeq<S, D, I>, source: &Vec<T>) -> bool {
        let mut source_iter = source.iter();
        for diff_node in &diff.0 {
            match diff_node {
                SpineSeqNode::Zipped(node) => {
                    let source_node = match source_iter.next() {
                        Some(n) => n,
                        None => return false,
                    };
                    if !self.can_merge(node, source_node) {
                        return false;
                    }
                }
                SpineSeqNode::Deleted(del) | SpineSeqNode::DeleteConflict(del, _) => {
                    let source_node = match source_iter.next() {
                        Some(n) => n,
                        None => return false,
                    };
                    if !self.can_merge(del, source_node) {
                        return false;
                    }
                }
                SpineSeqNode::Inserted(_) | SpineSeqNode::InsertOrderConflict(_) => (),
            }
        }
        source_iter.next().is_none()
    }

    fn merge(&mut self, diff: SpineSeq<S, D, I>, source: Vec<T>) -> SpineSeq<S, D, I> {
        let mut source_iter = source.into_iter();
        SpineSeq(
            diff.0
                .into_iter()
                .map(|diff_node| match diff_node {
                    SpineSeqNode::Zipped(node) => {
                        let source_node = source_iter.next().unwrap();
                        SpineSeqNode::Zipped(self.merge(node, source_node))
                    }
                    SpineSeqNode::Deleted(del) => {
                        let source_node = source_iter.next().unwrap();
                        SpineSeqNode::Deleted(self.merge(del, source_node))
                    }
                    SpineSeqNode::DeleteConflict(del, ins) => {
                        let source_node = source_iter.next().unwrap();
                        SpineSeqNode::DeleteConflict(self.merge(del, source_node), ins)
                    }
                    SpineSeqNode::Inserted(_) | SpineSeqNode::InsertOrderConflict(_) => diff_node,
                })
                .collect(),
        )
    }
}

impl<T> Merge<(), T, ()> for MetavarRemover {
    fn can_merge(&mut self, _: &(), _: &T) -> bool {
        true
    }

    fn merge(&mut self, _: (), _: T) {}
}

impl<T: ToString> Merge<String, T, String> for MetavarRemover {
    fn can_merge(&mut self, left: &String, right: &T) -> bool {
        *left == right.to_string()
    }

    fn merge(&mut self, left: String, _: T) -> String {
        left
    }
}

impl<T> VisitMut<Colored<T>> for MetavarRemover
where
    MetavarRemover: VisitMut<T>,
{
    fn visit_mut(&mut self, node: &mut Colored<T>) {
        self.visit_mut(&mut node.node)
    }
}

impl<I> VisitMut<InsNode<I>> for MetavarRemover
where
    MetavarRemover: VisitMut<Colored<I>>,
    InsNode<I>: Clone + 'static,
{
    fn visit_mut(&mut self, node: &mut InsNode<I>) {
        match node {
            InsNode::InPlace(ins) => self.visit_mut(ins),
            InsNode::Elided(mv) => {
                if self.metavar_replacements.len() <= mv.0 {
                    panic!("A metavariable appears in insertion but never in deletion");
                }
                if !self.metavar_conflict[mv.0] {
                    match &self.metavar_replacements[mv.0] {
                        None => panic!("A metavariable appears in insertion but never in deletion"),
                        Some(repl) => {
                            *node = repl.downcast_ref::<InsNode<I>>().unwrap().clone();
                        }
                    }
                }
            }
            InsNode::Conflict(ins_list) => {
                for ins in ins_list {
                    VisitMut::<InsNode<I>>::visit_mut(self, ins)
                }
            }
        }
    }
}

impl<I> VisitMut<InsSeq<I>> for MetavarRemover
where
    MetavarRemover: VisitMut<InsNode<I>>,
    MetavarRemover: VisitMut<Vec<Colored<Vec<InsNode<I>>>>>,
{
    fn visit_mut(&mut self, seq: &mut InsSeq<I>) {
        for node in &mut seq.0 {
            match node {
                InsSeqNode::Node(node) | InsSeqNode::DeleteConflict(node) => self.visit_mut(node),
                InsSeqNode::InsertOrderConflict(conflict) => self.visit_mut(conflict),
            }
        }
    }
}

impl<D, I> VisitMut<DelNode<D, I>> for MetavarRemover
where
    MetavarRemover: VisitMut<D>,
    MetavarRemover: VisitMut<InsNode<I>>,
{
    fn visit_mut(&mut self, node: &mut DelNode<D, I>) {
        match node {
            DelNode::InPlace(del) => self.visit_mut(del),
            DelNode::Elided(_) => panic!("A metavariable was not removed in deletion tree"),
            DelNode::MetavariableConflict(_, del, repl) => {
                VisitMut::<DelNode<D, I>>::visit_mut(self, del);
                match repl {
                    None => (),
                    Some(ins) => self.visit_mut(ins),
                }
            }
        }
    }
}

impl<S, D, I> VisitMut<SpineNode<S, D, I>> for MetavarRemover
where
    MetavarRemover: VisitMut<S>,
    MetavarRemover: VisitMut<DelNode<D, I>>,
    MetavarRemover: VisitMut<InsNode<I>>,
{
    fn visit_mut(&mut self, node: &mut SpineNode<S, D, I>) {
        match node {
            SpineNode::Spine(spine) => self.visit_mut(spine),
            SpineNode::Unchanged => panic!("An unchanged node not was not removed in the spine"),
            SpineNode::Changed(del, ins) => {
                self.visit_mut(del);
                self.visit_mut(ins);
            }
        }
    }
}

impl<S, D, I> VisitMut<SpineSeq<S, D, I>> for MetavarRemover
where
    MetavarRemover: VisitMut<SpineNode<S, D, I>>,
    MetavarRemover: VisitMut<DelNode<D, I>>,
    MetavarRemover: VisitMut<InsNode<I>>,
{
    fn visit_mut(&mut self, seq: &mut SpineSeq<S, D, I>) {
        for node in &mut seq.0 {
            match node {
                SpineSeqNode::Zipped(node) => self.visit_mut(node),
                SpineSeqNode::Deleted(del) => self.visit_mut(del),
                SpineSeqNode::DeleteConflict(del, ins) => {
                    self.visit_mut(del);
                    self.visit_mut(ins);
                }
                SpineSeqNode::Inserted(ins) => self.visit_mut(ins),
                SpineSeqNode::InsertOrderConflict(ins_conflict) => self.visit_mut(ins_conflict),
            }
        }
    }
}

pub struct InferFromSynColored(ColorSet);

impl<T, D, I> Convert<T, DelNode<D, I>> for InferFromSynColored
where
    InferFromSynColored: Convert<T, D>,
{
    fn convert(&mut self, node: T) -> DelNode<D, I> {
        DelNode::InPlace(Colored {
            node: self.convert(node),
            colors: self.0,
        })
    }
}

pub struct InferFromSyn;

impl<T, I> Convert<T, InsNode<I>> for InferFromSyn
where
    InferFromSyn: Convert<T, I>,
{
    fn convert(&mut self, node: T) -> InsNode<I> {
        InsNode::InPlace(Colored::new_white(self.convert(node)))
    }
}

impl<T, I> Convert<Vec<T>, InsSeq<I>> for InferFromSyn
where
    InferFromSyn: Convert<T, InsNode<I>>,
{
    fn convert(&mut self, node_seq: Vec<T>) -> InsSeq<I> {
        InsSeq(
            node_seq
                .into_iter()
                .map(|node| InsSeqNode::Node(self.convert(node)))
                .collect(),
        )
    }
}

impl<T, S, D, I> Convert<T, SpineNode<S, D, I>> for InferFromSyn
where
    InferFromSyn: Convert<T, S>,
{
    fn convert(&mut self, node: T) -> SpineNode<S, D, I> {
        SpineNode::Spine(self.convert(node))
    }
}

impl<T, S, D, I> Convert<Vec<T>, SpineSeq<S, D, I>> for InferFromSyn
where
    InferFromSyn: Convert<T, SpineNode<S, D, I>>,
{
    fn convert(&mut self, node_seq: Vec<T>) -> SpineSeq<S, D, I> {
        SpineSeq(
            node_seq
                .into_iter()
                .map(|node| SpineSeqNode::Zipped(self.convert(node)))
                .collect(),
        )
    }
}

pub fn remove_metavars<S, T>(multi_diff: S, source: T) -> Option<S>
where
    MetavarRemover: Merge<S, T, S>,
    MetavarRemover: VisitMut<S>,
{
    let mut remover = MetavarRemover {
        metavar_replacements: Vec::new(),
        metavar_conflict: Vec::new(),
        mergeable: true,
    };
    if remover.can_merge(&multi_diff, &source) {
        let mut multi_diff = remover.merge(multi_diff, source);
        remover.visit_mut(&mut multi_diff);
        if remover.mergeable {
            Some(multi_diff)
        } else {
            None
        }
    } else {
        None
    }
}
