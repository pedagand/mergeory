//use super::id_merger::{is_del_equivalent_to_ins, merge_id_ins};
use super::merge_ins::MetavarInsReplacementList;
use super::{
    DelNode, InsNode, MergedInsNode, MergedSpineNode, MergedSpineSeqNode, MetavarInsReplacement,
};
use crate::generic_tree::{Subtree, Tree};
use crate::{ColorSet, Colored, Metavariable};

enum ComputableSubst<T, U> {
    Pending(U),
    Processing,
    Computed(T),
}

struct Substituter<'t> {
    del_subst: Vec<ComputableSubst<DelNode<'t>, Option<DelNode<'t>>>>,
    ins_subst: Vec<ComputableSubst<Option<InsNode<'t>>, MetavarInsReplacementList<'t>>>,
    ins_cycle_stack: Vec<(Metavariable, bool)>,
}

impl<'t> Substituter<'t> {
    fn new(
        del_subst: Vec<Option<DelNode<'t>>>,
        ins_subst: Vec<MetavarInsReplacementList<'t>>,
    ) -> Self {
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
    fn find_del_subst(&mut self, mv: Metavariable) -> DelNode<'t> {
        let repl = match std::mem::replace(&mut self.del_subst[mv.0], ComputableSubst::Processing) {
            ComputableSubst::Computed(repl_del) => repl_del,
            ComputableSubst::Pending(None) => DelNode::Elided(Colored::new_white(mv)),
            ComputableSubst::Pending(Some(mut repl_del)) => {
                self.substitute_in_del_node(&mut repl_del);
                repl_del
            }
            ComputableSubst::Processing => {
                // In del_subst cycles can only occur between metavariables that should be all
                // unified together. Break the cycle by behaving once as identity.
                DelNode::Elided(Colored::new_white(mv))
            }
        };
        self.del_subst[mv.0] = ComputableSubst::Computed(repl.clone());
        repl
    }

    fn find_ins_subst(&mut self, mv: Metavariable) -> InsNode<'t> {
        let subst = match std::mem::replace(&mut self.ins_subst[mv.0], ComputableSubst::Processing)
        {
            ComputableSubst::Computed(subst) => subst,
            ComputableSubst::Pending(replacements) => {
                self.ins_cycle_stack.push((mv, false));
                let mut repl_ins = replacements
                    .into_iter()
                    .map(|repl| match repl {
                        MetavarInsReplacement::InferFromDel => {
                            // Build the insertion substitution from the deletion substitution
                            infer_ins_from_del(&self.find_del_subst(mv))
                        }
                        MetavarInsReplacement::Inlined(mut ins) => {
                            self.substitute_in_ins_node(&mut ins);
                            ins
                        }
                    })
                    .collect::<Vec<_>>();
                let (cycle_mv, cycle) = self.ins_cycle_stack.pop().unwrap();
                assert!(cycle_mv == mv);
                if !cycle {
                    // No cycle during computation on potential replacements, try to fuse them
                    let last_ins = repl_ins.pop().unwrap();
                    repl_ins
                        .into_iter()
                        .try_fold(last_ins, |acc, ins| merge_id_ins(&acc, &ins))
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
                self.ins_subst[mv.0] = ComputableSubst::Computed(Some(subst.clone()));
                subst
            }
            None => {
                // Save conflict and return a simple white metavariable
                self.ins_subst[mv.0] = ComputableSubst::Computed(None);
                InsNode::Elided(Colored::new_white(mv))
            }
        }
    }

    fn substitute_in_del_node(&mut self, node: &mut DelNode<'t>) {
        match node {
            DelNode::InPlace(del) => del
                .data
                .visit_mut(|sub| self.substitute_in_del_node(&mut sub.node)),
            DelNode::Elided(mv) => {
                let mut subst = self.find_del_subst(mv.data);
                replace_colors(&mut subst, mv.colors);
                *node = subst;
            }
            DelNode::MetavariableConflict(_, del, _) => {
                // The insertion replacement part will be visited only if the conflict stays
                self.substitute_in_del_node(del)
            }
        }
    }

    fn substitute_in_ins_node(&mut self, node: &mut InsNode<'t>) {
        match node {
            InsNode::InPlace(ins) => ins
                .data
                .visit_mut(|sub| self.substitute_in_ins_node(&mut sub.node)),
            InsNode::Elided(mv) => *node = self.find_ins_subst(mv.data),
        }
    }

    fn substitute_in_merged_ins_node(&mut self, node: &mut MergedInsNode<'t>) {
        match node {
            MergedInsNode::InPlace(ins) => ins
                .data
                .visit_mut(|sub| self.substitute_in_merged_ins_node(&mut sub.node)),
            MergedInsNode::Elided(mv) => {
                *node = MergedInsNode::from_simple_ins(self.find_ins_subst(mv.data))
            }
            MergedInsNode::Conflict(left_ins, right_ins) => {
                self.substitute_in_ins_node(left_ins);
                self.substitute_in_ins_node(right_ins);

                // Try to solve the insertion conflict after substitution
                match merge_id_ins(&left_ins, &right_ins) {
                    Some(merged_ins) => *node = MergedInsNode::from_simple_ins(merged_ins),
                    None => (),
                }
            }
        }
    }

    fn substitute_in_spine_node(&mut self, node: &mut MergedSpineNode<'t>) {
        match node {
            MergedSpineNode::Spine(spine) => match spine {
                Tree::Node(_, children) => self.substitute_in_spine_seq(children),
                Tree::Leaf(_) => (),
            },
            MergedSpineNode::Unchanged => (),
            MergedSpineNode::Changed(del, ins) => {
                self.substitute_in_del_node(del);
                self.substitute_in_merged_ins_node(ins);
            }
        }
    }

    fn substitute_in_spine_seq(&mut self, seq: &mut Vec<MergedSpineSeqNode<'t>>) {
        for node in std::mem::take(seq) {
            match node {
                MergedSpineSeqNode::Zipped(mut spine) => {
                    self.substitute_in_spine_node(&mut spine.node);
                    seq.push(MergedSpineSeqNode::Zipped(spine))
                }
                MergedSpineSeqNode::Deleted(mut del_seq) => {
                    for del in &mut del_seq {
                        self.substitute_in_del_node(&mut del.node)
                    }
                    if let Some(MergedSpineSeqNode::Deleted(prev_del_seq)) = seq.last_mut() {
                        prev_del_seq.extend(del_seq)
                    } else {
                        seq.push(MergedSpineSeqNode::Deleted(del_seq))
                    }
                }
                MergedSpineSeqNode::DeleteConflict(field, mut del, mut ins) => {
                    self.substitute_in_del_node(&mut del);
                    self.substitute_in_ins_node(&mut ins);

                    // Solve the delete conflict if del and ins are identical after substitution
                    if is_del_equivalent_to_ins(&del, &ins) {
                        let del_subtree = Subtree { field, node: del };
                        if let Some(MergedSpineSeqNode::Deleted(prev_del_seq)) = seq.last_mut() {
                            prev_del_seq.push(del_subtree)
                        } else {
                            seq.push(MergedSpineSeqNode::Deleted(vec![del_subtree]))
                        }
                    } else {
                        seq.push(MergedSpineSeqNode::DeleteConflict(field, del, ins))
                    }
                }
                MergedSpineSeqNode::Inserted(mut ins_seq) => {
                    for ins in &mut ins_seq.data {
                        self.substitute_in_ins_node(&mut ins.node)
                    }
                    seq.push(MergedSpineSeqNode::Inserted(ins_seq))
                }
                MergedSpineSeqNode::InsertOrderConflict(mut left_ins_seq, mut right_ins_seq) => {
                    for ins_seq in [&mut left_ins_seq, &mut right_ins_seq] {
                        for ins in &mut ins_seq.data {
                            self.substitute_in_ins_node(&mut ins.node)
                        }
                    }

                    // Try to resolve the merge conflict after the substitutions
                    match Colored::merge(
                        left_ins_seq.as_ref(),
                        right_ins_seq.as_ref(),
                        |left, right| {
                            if left.len() != right.len() {
                                return None;
                            }

                            left.iter()
                                .zip(right)
                                .map(|(l, r)| Subtree::merge(l.as_ref(), r.as_ref(), merge_id_ins))
                                .collect()
                        },
                    ) {
                        Some(merged_ins_seq) => {
                            seq.push(MergedSpineSeqNode::Inserted(merged_ins_seq))
                        }
                        None => seq.push(MergedSpineSeqNode::InsertOrderConflict(
                            left_ins_seq,
                            right_ins_seq,
                        )),
                    }
                }
            }
        }
    }

    fn remove_solved_conflicts_in_del(&mut self, node: &mut DelNode<'t>) {
        match node {
            DelNode::InPlace(del) => del
                .data
                .visit_mut(|sub| self.remove_solved_conflicts_in_del(&mut sub.node)),
            DelNode::Elided(_) => (),
            DelNode::MetavariableConflict(mv, del, repl) => {
                self.remove_solved_conflicts_in_del(del);
                match &self.ins_subst[mv.0] {
                    ComputableSubst::Computed(Some(_)) => {
                        *node = std::mem::replace(del, DelNode::Elided(Colored::new_white(*mv)))
                    }
                    ComputableSubst::Computed(None) => match repl {
                        MetavarInsReplacement::InferFromDel => (),
                        MetavarInsReplacement::Inlined(ins) => self.substitute_in_ins_node(ins),
                    },
                    ComputableSubst::Pending(_) => {
                        match repl {
                            MetavarInsReplacement::InferFromDel => {
                                *node =
                                    std::mem::replace(del, DelNode::Elided(Colored::new_white(*mv)))
                            }
                            MetavarInsReplacement::Inlined(ins) => {
                                // We are dealing here with a subtree inlined into a metavariable
                                // that was never inserted back.
                                // Removing the conflict would arbitrarily drop a modification so
                                // we keep the metavariable conflict.
                                // We might accidentally call this too late and consider unused a
                                // metavariable replacement used inside another metavariable
                                // conflict, but all other solutions seem worse.
                                self.substitute_in_ins_node(ins)
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

    fn remove_solved_conflicts_in_spine_node(&mut self, node: &mut MergedSpineNode<'t>) {
        match node {
            MergedSpineNode::Spine(spine) => {
                spine.visit_mut(|sub| self.remove_solved_conflicts_in_spine_seq_node(sub))
            }
            MergedSpineNode::Unchanged => (),
            MergedSpineNode::Changed(del, _) => self.remove_solved_conflicts_in_del(del),
        }
    }

    fn remove_solved_conflicts_in_spine_seq_node(&mut self, node: &mut MergedSpineSeqNode<'t>) {
        match node {
            MergedSpineSeqNode::Zipped(spine) => {
                self.remove_solved_conflicts_in_spine_node(&mut spine.node)
            }
            MergedSpineSeqNode::Deleted(del_list) => {
                for del in del_list {
                    self.remove_solved_conflicts_in_del(&mut del.node)
                }
            }
            MergedSpineSeqNode::DeleteConflict(_, del, _) => {
                self.remove_solved_conflicts_in_del(del)
            }
            MergedSpineSeqNode::Inserted(_) | MergedSpineSeqNode::InsertOrderConflict(..) => (),
        }
    }
}

fn replace_colors(node: &mut DelNode, colors: ColorSet) {
    match node {
        DelNode::InPlace(del) => {
            del.data
                .visit_mut(|sub| replace_colors(&mut sub.node, colors));
            del.colors = colors;
        }
        DelNode::Elided(mv) => mv.colors = colors,
        DelNode::MetavariableConflict(_, del, _) => replace_colors(del, colors),
    }
}

fn infer_ins_from_del<'t>(del: &DelNode<'t>) -> InsNode<'t> {
    match del {
        DelNode::InPlace(del) => InsNode::InPlace(Colored::new_white(
            del.data
                .map_children(|child| child.as_ref().map(infer_ins_from_del)),
        )),
        DelNode::Elided(mv) => InsNode::Elided(Colored::new_white(mv.data)),
        DelNode::MetavariableConflict(_, del, _) => infer_ins_from_del(del),
    }
}

pub fn merge_id_ins<'t>(left: &InsNode<'t>, right: &InsNode<'t>) -> Option<InsNode<'t>> {
    match (left, right) {
        (InsNode::InPlace(left), InsNode::InPlace(right)) => Some(InsNode::InPlace(
            Colored::merge(left.as_ref(), right.as_ref(), |l, r| {
                Tree::merge_subtrees_to(l, r, merge_id_ins)
            })?,
        )),
        (InsNode::Elided(left), InsNode::Elided(right)) => Some(InsNode::Elided(Colored::merge(
            *left,
            *right,
            |left_mv, right_mv| {
                if left_mv == right_mv {
                    Some(left_mv)
                } else {
                    None
                }
            },
        )?)),
        _ => None,
    }
}

fn is_del_equivalent_to_ins(del: &DelNode, ins: &InsNode) -> bool {
    match (del, ins) {
        (DelNode::InPlace(del), InsNode::InPlace(ins)) => {
            Tree::compare_subtrees(&del.data, &ins.data, is_del_equivalent_to_ins)
        }
        (DelNode::Elided(del_mv), InsNode::Elided(ins_mv)) => del_mv.data == ins_mv.data,
        (DelNode::MetavariableConflict(_, del, _), ins) => is_del_equivalent_to_ins(del, ins),
        (DelNode::InPlace(_), InsNode::Elided(_)) | (DelNode::Elided(_), InsNode::InPlace(_)) => {
            false
        }
    }
}

pub fn apply_metavar_substitutions<'t>(
    tree: &mut MergedSpineNode<'t>,
    del_subst: Vec<Option<DelNode<'t>>>,
    ins_subst: Vec<MetavarInsReplacementList<'t>>,
) {
    let mut subst = Substituter::new(del_subst, ins_subst);
    subst.substitute_in_spine_node(tree);
    subst.remove_solved_conflicts_in_spine_node(tree);
}
