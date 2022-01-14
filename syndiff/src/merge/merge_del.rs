use super::merge_ins::{InsMergedSpineNode, InsMergedSpineSeqNode};
use super::{DelNode, MergedSpineNode, MergedSpineSeqNode};
use crate::generic_tree::{Subtree, Tree};
use crate::{ColorSet, Colored};

fn merge_del_nodes<'t>(
    left: DelNode<'t>,
    right: DelNode<'t>,
    metavars_del: &mut [Option<DelNode<'t>>],
) -> Option<DelNode<'t>> {
    Some(match (left, right) {
        (DelNode::InPlace(left), DelNode::InPlace(right)) => {
            DelNode::InPlace(Colored::merge(left, right, |left, right| {
                Tree::merge_subtrees_into(left, right, |l, r| merge_del_nodes(l, r, metavars_del))
            })?)
        }
        (DelNode::MetavariableConflict(mv, del, ins), other)
        | (other, DelNode::MetavariableConflict(mv, del, ins)) => {
            let new_del = merge_del_nodes(*del, other, metavars_del)?;
            DelNode::MetavariableConflict(mv, Box::new(new_del), ins)
        }
        (DelNode::Elided(mv), DelNode::Elided(other_mv)) if mv.data == other_mv.data => {
            // This case can occur because of unification inside metavars_del
            DelNode::Elided(Colored {
                data: mv.data,
                colors: mv.colors | other_mv.colors,
            })
        }
        (DelNode::Elided(mv), mut other) | (mut other, DelNode::Elided(mv)) => {
            let mv_id = mv.data.0;
            match metavars_del[mv_id].take() {
                Some(repl_tree) => {
                    // Perform unification on the metavariable del replacement
                    let new_repl_tree = merge_del_nodes(repl_tree, other.clone(), metavars_del)?;
                    metavars_del[mv_id] = Some(new_repl_tree)
                }
                None => metavars_del[mv_id] = Some(other.clone()),
            }

            // Keep other in tree to retain its colors
            add_colors(mv.colors, &mut other);
            other
        }
    })
}

fn merge_del_in_spine<'t>(
    spine: InsMergedSpineNode<'t>,
    metavars_del: &mut [Option<DelNode<'t>>],
) -> Option<MergedSpineNode<'t>> {
    Some(match spine {
        InsMergedSpineNode::Spine(s) => MergedSpineNode::Spine(
            s.try_convert_into(|ch| merge_del_in_spine_seq(ch, metavars_del))?,
        ),
        InsMergedSpineNode::Unchanged => MergedSpineNode::Unchanged,
        InsMergedSpineNode::OneChange(del, ins) => MergedSpineNode::Changed(del, ins),
        InsMergedSpineNode::BothChanged(left_del, right_del, ins) => {
            MergedSpineNode::Changed(merge_del_nodes(left_del, right_del, metavars_del)?, ins)
        }
    })
}

fn merge_del_in_spine_seq<'t>(
    spine_seq: Vec<InsMergedSpineSeqNode<'t>>,
    metavars_del: &mut [Option<DelNode<'t>>],
) -> Option<Vec<MergedSpineSeqNode<'t>>> {
    let mut merged_vec = Vec::new();
    for seq_node in spine_seq {
        match seq_node {
            InsMergedSpineSeqNode::Zipped(node) => merged_vec.push(MergedSpineSeqNode::Zipped(
                node.try_map(|node| merge_del_in_spine(node, metavars_del))?,
            )),
            InsMergedSpineSeqNode::BothDeleted(field, left_del, right_del) => {
                let del = Subtree {
                    field,
                    node: merge_del_nodes(left_del, right_del, metavars_del)?,
                };
                if let Some(MergedSpineSeqNode::Deleted(del_list)) = merged_vec.last_mut() {
                    del_list.push(del);
                } else {
                    merged_vec.push(MergedSpineSeqNode::Deleted(vec![del]));
                }
            }
            InsMergedSpineSeqNode::DeleteConflict(field, left_del, right_del, ins) => merged_vec
                .push(MergedSpineSeqNode::DeleteConflict(
                    field,
                    merge_del_nodes(left_del, right_del, metavars_del)?,
                    ins,
                )),
            InsMergedSpineSeqNode::Inserted(ins_list) => {
                merged_vec.push(MergedSpineSeqNode::Inserted(ins_list))
            }
            InsMergedSpineSeqNode::InsertOrderConflict(left_ins, right_ins) => {
                merged_vec.push(MergedSpineSeqNode::InsertOrderConflict(left_ins, right_ins))
            }
        }
    }
    Some(merged_vec)
}

fn add_colors(colors: ColorSet, node: &mut DelNode) {
    match node {
        DelNode::InPlace(del) => {
            del.data.visit_mut(|ch| add_colors(colors, &mut ch.node));
            del.colors |= colors
        }
        DelNode::Elided(mv) => mv.colors |= colors,
        DelNode::MetavariableConflict(_, del, _) => add_colors(colors, del),
    }
}

pub fn merge_del(
    input: InsMergedSpineNode,
    nb_metavars: usize,
) -> Option<(MergedSpineNode, Vec<Option<DelNode>>)> {
    let mut metavars_del = Vec::new();
    metavars_del.resize_with(nb_metavars, || None);
    let output = merge_del_in_spine(input, &mut metavars_del)?;
    Some((output, metavars_del))
}
