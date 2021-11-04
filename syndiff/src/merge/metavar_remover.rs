use super::id_merger::merge_id_ins;
use super::{
    Colored, DelNode, InsNode, InsSeqNode, MetavarInsReplacement, SpineNode, SpineSeqNode,
};
use crate::generic_tree::{Subtree, Tree};
use crate::SynNode;

struct MetavarRemover<'t> {
    metavar_replacements: Vec<Option<InsNode<'t>>>,
    metavar_conflict: Vec<bool>,
}

fn merge_with_syn<'t, T>(
    tree: Tree<'t, T>,
    source: &SynNode<'t>,
    merge_child_fn: impl FnOnce(Vec<T>, &[Subtree<SynNode<'t>>]) -> Option<Vec<T>>,
) -> Option<Tree<'t, T>> {
    match (tree, &source.0) {
        (Tree::Node(tree_kind, tree_ch), Tree::Node(source_kind, source_ch))
            if tree_kind == *source_kind =>
        {
            Some(Tree::Node(tree_kind, merge_child_fn(tree_ch, source_ch)?))
        }
        (Tree::Leaf(tree_tok), Tree::Leaf(source_tok)) if tree_tok == *source_tok => {
            Some(Tree::Leaf(tree_tok))
        }
        _ => None,
    }
}

impl<'t> MetavarRemover<'t> {
    fn remove_metavars_in_del_node(
        &mut self,
        del: DelNode<'t>,
        source: &SynNode<'t>,
    ) -> Option<DelNode<'t>> {
        Some(match del {
            DelNode::InPlace(d) => DelNode::InPlace(Colored {
                node: merge_with_syn(d.node, source, |del_ch, src_ch| {
                    if del_ch.len() != src_ch.len() {
                        return None;
                    }
                    del_ch
                        .into_iter()
                        .zip(src_ch)
                        .map(|(d, s)| {
                            Subtree::merge(d, s.as_ref(), |d, s| {
                                self.remove_metavars_in_del_node(d, s)
                            })
                        })
                        .collect()
                })?,
                colors: d.colors,
            }),
            DelNode::Elided(mv) => {
                let mv_id = mv.node.0;
                if self.metavar_replacements.len() <= mv_id {
                    self.metavar_replacements
                        .resize_with(mv_id + 1, Default::default);
                    self.metavar_conflict.resize(mv_id + 1, false);
                }

                let ins_repl = InsNode::from_syn(source);
                match &self.metavar_replacements[mv_id] {
                    None => self.metavar_replacements[mv_id] = Some(ins_repl),
                    Some(cur_repl) => {
                        merge_id_ins(cur_repl, &ins_repl)?;
                    }
                }
                DelNode::from_syn(source, mv.colors)
            }
            DelNode::MetavariableConflict(mv, del, repl) => {
                if self.metavar_replacements.len() <= mv.0 {
                    self.metavar_replacements
                        .resize_with(mv.0 + 1, Default::default);
                    self.metavar_conflict.resize(mv.0 + 1, false);
                }
                self.metavar_conflict[mv.0] = true;
                DelNode::MetavariableConflict(
                    mv,
                    Box::new(self.remove_metavars_in_del_node(*del, source)?),
                    repl,
                )
            }
        })
    }

    fn remove_metavars_in_spine_node(
        &mut self,
        diff: SpineNode<'t>,
        source: &SynNode<'t>,
    ) -> Option<SpineNode<'t>> {
        Some(match diff {
            SpineNode::Spine(spine) => {
                SpineNode::Spine(merge_with_syn(spine, source, |spine_ch, source_ch| {
                    self.remove_metavars_in_spine_seq(spine_ch, source_ch)
                })?)
            }
            SpineNode::Unchanged => SpineNode::from_syn(source),
            SpineNode::Changed(del, ins) => {
                SpineNode::Changed(self.remove_metavars_in_del_node(del, source)?, ins)
            }
        })
    }

    fn remove_metavars_in_spine_seq(
        &mut self,
        spine_seq: Vec<SpineSeqNode<'t>>,
        source_seq: &[Subtree<SynNode<'t>>],
    ) -> Option<Vec<SpineSeqNode<'t>>> {
        let mut source_iter = source_seq.iter();
        let result_seq = spine_seq
            .into_iter()
            .map(|diff_node| {
                Some(match diff_node {
                    SpineSeqNode::Zipped(node) => {
                        let source_node = source_iter.next()?;
                        SpineSeqNode::Zipped(Subtree::merge(
                            node,
                            source_node.as_ref(),
                            |diff, source| self.remove_metavars_in_spine_node(diff, source),
                        )?)
                    }
                    SpineSeqNode::Deleted(del_list) => SpineSeqNode::Deleted(
                        del_list
                            .into_iter()
                            .map(|del| {
                                let source_node = source_iter.next()?;
                                Subtree::merge(del, source_node.as_ref(), |del, source| {
                                    self.remove_metavars_in_del_node(del, source)
                                })
                            })
                            .collect::<Option<_>>()?,
                    ),
                    SpineSeqNode::DeleteConflict(field, del, ins) => {
                        let source_node = source_iter.next()?;
                        if source_node.field != field {
                            return None;
                        }
                        SpineSeqNode::DeleteConflict(
                            field,
                            self.remove_metavars_in_del_node(del, &source_node.node)?,
                            ins,
                        )
                    }
                    SpineSeqNode::Inserted(_) | SpineSeqNode::InsertOrderConflict(_) => diff_node,
                })
            })
            .collect();

        // Check that we have taken all the source nodes
        if source_iter.next().is_none() {
            result_seq
        } else {
            None
        }
    }

    fn replace_metavars_in_ins_node(&self, node: &mut InsNode<'t>) {
        match node {
            InsNode::InPlace(ins) => ins
                .node
                .visit_mut(|ch| self.replace_metavars_in_ins_seq_node(ch)),
            InsNode::Elided(mv) => {
                if self.metavar_replacements.len() <= mv.0 {
                    panic!("A metavariable appears in insertion but never in deletion");
                }
                if !self.metavar_conflict[mv.0] {
                    match &self.metavar_replacements[mv.0] {
                        None => panic!("A metavariable appears in insertion but never in deletion"),
                        Some(repl) => *node = repl.clone(),
                    }
                }
            }
            InsNode::Conflict(ins_list) => {
                for ins in ins_list {
                    self.replace_metavars_in_ins_node(ins)
                }
            }
        }
    }

    fn replace_metavars_in_ins_seq_node(&self, node: &mut InsSeqNode<'t>) {
        match node {
            InsSeqNode::Node(node) | InsSeqNode::DeleteConflict(node) => {
                self.replace_metavars_in_ins_node(&mut node.node)
            }
            InsSeqNode::InsertOrderConflict(conflict) => {
                for ins_list in conflict {
                    for ins in &mut ins_list.node {
                        self.replace_metavars_in_ins_node(&mut ins.node)
                    }
                }
            }
        }
    }

    fn replace_metavars_in_del_node(&self, node: &mut DelNode<'t>) {
        match node {
            DelNode::InPlace(del) => del
                .node
                .visit_mut(|ch| self.replace_metavars_in_del_node(&mut ch.node)),
            DelNode::Elided(_) => panic!("A metavariable was not removed in deletion tree"),
            DelNode::MetavariableConflict(_, del, repl) => {
                self.replace_metavars_in_del_node(del);
                match repl {
                    MetavarInsReplacement::InferFromDel => (),
                    MetavarInsReplacement::Inlined(ins) => self.replace_metavars_in_ins_node(ins),
                }
            }
        }
    }

    fn replace_metavars_in_spine_node(&self, node: &mut SpineNode<'t>) {
        match node {
            SpineNode::Spine(spine) => {
                spine.visit_mut(|ch| self.replace_metavars_in_spine_seq_node(ch))
            }
            SpineNode::Unchanged => panic!("An unchanged node not was not removed in the spine"),
            SpineNode::Changed(del, ins) => {
                self.replace_metavars_in_del_node(del);
                self.replace_metavars_in_ins_node(ins);
            }
        }
    }

    fn replace_metavars_in_spine_seq_node(&self, node: &mut SpineSeqNode<'t>) {
        match node {
            SpineSeqNode::Zipped(node) => self.replace_metavars_in_spine_node(&mut node.node),
            SpineSeqNode::Deleted(del_list) => {
                for del in del_list {
                    self.replace_metavars_in_del_node(&mut del.node)
                }
            }
            SpineSeqNode::DeleteConflict(_, del, ins) => {
                self.replace_metavars_in_del_node(del);
                self.replace_metavars_in_ins_node(ins);
            }
            SpineSeqNode::Inserted(ins_list) => {
                for ins in &mut ins_list.node {
                    self.replace_metavars_in_ins_node(&mut ins.node)
                }
            }
            SpineSeqNode::InsertOrderConflict(ins_conflict) => {
                for ins_list in ins_conflict {
                    for ins in &mut ins_list.node {
                        self.replace_metavars_in_ins_node(&mut ins.node)
                    }
                }
            }
        }
    }
}

pub fn remove_metavars<'t>(diff: SpineNode<'t>, source: &SynNode<'t>) -> Option<SpineNode<'t>> {
    let mut remover = MetavarRemover {
        metavar_replacements: Vec::new(),
        metavar_conflict: Vec::new(),
    };
    remover
        .remove_metavars_in_spine_node(diff, source)
        .map(|mut tree| {
            remover.replace_metavars_in_spine_node(&mut tree);
            tree
        })
}
