use super::merge_ins::{ISpineNode, ISpineSeq, ISpineSeqNode};
use super::{ColorSet, Colored, DelNode, SpineNode, SpineSeq, SpineSeqNode};
use crate::family_traits::{Convert, Merge, VisitMut};
use std::any::Any;

pub struct DelMerger {
    metavars_del: Vec<Option<Box<dyn Any>>>,
    mergeable: bool,
}

impl<D, I> Merge<DelNode<D, I>, DelNode<D, I>, DelNode<D, I>> for DelMerger
where
    DelMerger: Merge<Colored<D>, Colored<D>, Colored<D>>,
    DelNode<D, I>: Clone + 'static,
    ColorAdder: VisitMut<DelNode<D, I>>,
{
    fn can_merge(&mut self, left: &DelNode<D, I>, right: &DelNode<D, I>) -> bool {
        // Here just compare the in place nodes, without caring about unification problems
        match (left, right) {
            (DelNode::InPlace(left_subtree), DelNode::InPlace(right_subtree)) => {
                self.can_merge(left_subtree, right_subtree)
            }
            (DelNode::MetavariableConflict(_, del, _), other)
            | (other, DelNode::MetavariableConflict(_, del, _)) => {
                Merge::<DelNode<D, I>, _, _>::can_merge(self, del, other)
            }
            (DelNode::Ellided(_), _) | (_, DelNode::Ellided(_)) => true,
        }
    }

    fn merge(&mut self, left: DelNode<D, I>, right: DelNode<D, I>) -> DelNode<D, I> {
        match (left, right) {
            (DelNode::InPlace(left_subtree), DelNode::InPlace(right_subtree)) => {
                DelNode::InPlace(self.merge(left_subtree, right_subtree))
            }
            (DelNode::MetavariableConflict(mv, del, ins), other)
            | (other, DelNode::MetavariableConflict(mv, del, ins)) => {
                let new_del = Merge::<DelNode<D, I>, _, _>::merge(self, *del, other);
                DelNode::MetavariableConflict(mv, Box::new(new_del), ins)
            }
            (DelNode::Ellided(mv), DelNode::Ellided(other_mv)) if mv.node == other_mv.node => {
                DelNode::Ellided(Colored {
                    node: mv.node,
                    colors: mv.colors | other_mv.colors,
                })
            }
            (DelNode::Ellided(mv), mut other) | (mut other, DelNode::Ellided(mv)) => {
                let mv_id = mv.node.0;
                match self.metavars_del[mv_id].take() {
                    Some(repl_tree) => {
                        let repl_tree = *repl_tree.downcast::<DelNode<D, I>>().unwrap();
                        if Merge::<DelNode<D, I>, _, _>::can_merge(self, &repl_tree, &other) {
                            let new_repl_tree =
                                Merge::<DelNode<D, I>, _, _>::merge(self, repl_tree, other.clone());
                            assert!(self.metavars_del[mv_id].is_none()); // Unification cycle
                            self.metavars_del[mv_id] = Some(Box::new(new_repl_tree))
                        } else {
                            // Unification failure
                            self.mergeable = false
                        }
                    }
                    None => self.metavars_del[mv_id] = Some(Box::new(other.clone())),
                }

                // Keep other in tree to retain its colors
                ColorAdder(mv.colors).visit_mut(&mut other);
                other
            }
        }
    }
}

impl<IS, S, D, I> Convert<ISpineNode<IS, D, I>, SpineNode<S, D, I>> for DelMerger
where
    DelMerger: Convert<IS, S>,
    DelMerger: Merge<DelNode<D, I>, DelNode<D, I>, DelNode<D, I>>,
{
    fn convert(&mut self, input: ISpineNode<IS, D, I>) -> SpineNode<S, D, I> {
        match input {
            ISpineNode::Spine(s) => SpineNode::Spine(self.convert(s)),
            ISpineNode::Unchanged => SpineNode::Unchanged,
            ISpineNode::OneChange(del, ins) => SpineNode::Changed(del, ins),
            ISpineNode::BothChanged(left_del, right_del, ins) => {
                if self.can_merge(&left_del, &right_del) {
                    SpineNode::Changed(self.merge(left_del, right_del), ins)
                } else {
                    self.mergeable = false;
                    SpineNode::Unchanged // We need to return something but it is meaningless
                }
            }
        }
    }
}

impl<IS, S, D, I> Convert<ISpineSeq<IS, D, I>, SpineSeq<S, D, I>> for DelMerger
where
    DelMerger: Convert<ISpineNode<IS, D, I>, SpineNode<S, D, I>>,
    DelMerger: Merge<DelNode<D, I>, DelNode<D, I>, DelNode<D, I>>,
{
    fn convert(&mut self, input: ISpineSeq<IS, D, I>) -> SpineSeq<S, D, I> {
        SpineSeq(
            input
                .0
                .into_iter()
                .map(|seq_node| match seq_node {
                    ISpineSeqNode::Zipped(node) => SpineSeqNode::Zipped(self.convert(node)),
                    ISpineSeqNode::BothDeleted(left_del, right_del) => {
                        if self.can_merge(&left_del, &right_del) {
                            SpineSeqNode::Deleted(self.merge(left_del, right_del))
                        } else {
                            self.mergeable = false;
                            SpineSeqNode::Zipped(SpineNode::Unchanged)
                        }
                    }
                    ISpineSeqNode::DeleteConflict(left_del, right_del, ins) => {
                        if self.can_merge(&left_del, &right_del) {
                            SpineSeqNode::DeleteConflict(self.merge(left_del, right_del), ins)
                        } else {
                            self.mergeable = false;
                            SpineSeqNode::Zipped(SpineNode::Unchanged)
                        }
                    }
                    ISpineSeqNode::Insert(mut ins_vec) => {
                        if ins_vec.len() == 1 {
                            SpineSeqNode::Inserted(ins_vec.pop().unwrap())
                        } else {
                            SpineSeqNode::InsertOrderConflict(ins_vec)
                        }
                    }
                })
                .collect(),
        )
    }
}

pub struct ColorAdder(ColorSet);

impl<D, I> VisitMut<DelNode<D, I>> for ColorAdder
where
    ColorAdder: VisitMut<D>,
{
    fn visit_mut(&mut self, node: &mut DelNode<D, I>) {
        match node {
            DelNode::InPlace(del) => {
                self.visit_mut(&mut del.node);
                del.colors |= self.0
            }
            DelNode::Ellided(mv) => mv.colors |= self.0,
            DelNode::MetavariableConflict(_, del, _) => {
                VisitMut::<DelNode<D, I>>::visit_mut(self, del)
            }
        }
    }
}

pub fn merge_del<I, O>(input: I, nb_metavars: usize) -> Option<(O, Vec<Option<Box<dyn Any>>>)>
where
    DelMerger: Convert<I, O>,
{
    let mut merger = DelMerger {
        metavars_del: Vec::new(),
        mergeable: true,
    };
    merger.metavars_del.resize_with(nb_metavars, || None);
    let output = merger.convert(input);
    if merger.mergeable {
        Some((output, merger.metavars_del))
    } else {
        None
    }
}
