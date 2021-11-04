use super::metavar_remover::remove_metavars;
use super::{InsNode, InsSeqNode, SpineNode, SpineSeqNode};
use crate::generic_tree::Subtree;
use crate::SynNode;

fn standalone_ins_to_syn(node: InsNode) -> Option<SynNode> {
    match node {
        InsNode::InPlace(ins) => Some(SynNode(
            ins.node.try_convert_into(standalone_ins_seq_to_syn)?,
        )),
        _ => None,
    }
}

fn standalone_ins_seq_to_syn(seq: Vec<InsSeqNode>) -> Option<Vec<Subtree<SynNode>>> {
    seq.into_iter()
        .map(|node| match node {
            InsSeqNode::Node(n) => n.try_map(standalone_ins_to_syn),
            _ => None,
        })
        .collect()
}

fn keep_only_ins_from_standalone_spine(spine: SpineNode) -> Option<SynNode> {
    match spine {
        SpineNode::Spine(spine) => Some(SynNode(
            spine.try_convert_into(keep_only_ins_from_standalone_spine_seq)?,
        )),
        SpineNode::Unchanged => None,
        SpineNode::Changed(_, ins) => standalone_ins_to_syn(ins),
    }
}

fn keep_only_ins_from_standalone_spine_seq<'t>(
    seq: Vec<SpineSeqNode<'t>>,
) -> Option<Vec<Subtree<SynNode<'t>>>> {
    seq.into_iter()
        .flat_map::<Box<dyn Iterator<Item = Option<Subtree<SynNode<'t>>>>>, _>(|node| match node {
            SpineSeqNode::Zipped(spine) => Box::new(std::iter::once(
                spine.try_map(keep_only_ins_from_standalone_spine),
            )),
            SpineSeqNode::Deleted(_) => Box::new(std::iter::empty()),
            SpineSeqNode::Inserted(ins_list) => Box::new(
                ins_list
                    .node
                    .into_iter()
                    .map(|sub| sub.try_map(standalone_ins_to_syn)),
            ),
            _ => Box::new(std::iter::once(None)),
        })
        .collect()
}

pub fn apply_patch<'t>(diff: SpineNode<'t>, source: &SynNode<'t>) -> Option<SynNode<'t>> {
    let standalone_diff = remove_metavars(diff, source)?;
    keep_only_ins_from_standalone_spine(standalone_diff)
}
