use super::metavar_remover::remove_metavars;
use super::{InsNode, MergedInsNode, MergedSpineNode, MergedSpineSeqNode};
use crate::generic_tree::Subtree;
use crate::SynNode;

fn standalone_ins_to_syn(node: InsNode) -> Option<SynNode> {
    match node {
        InsNode::InPlace(ins) => Some(SynNode(ins.data.try_convert_into(|ch| {
            ch.into_iter()
                .map(|sub| sub.try_map(standalone_ins_to_syn))
                .collect()
        })?)),
        _ => None,
    }
}

fn standalone_merged_ins_to_syn(node: MergedInsNode) -> Option<SynNode> {
    match node {
        MergedInsNode::InPlace(ins) => Some(SynNode(ins.try_convert_into(|ch| {
            ch.into_iter()
                .map(|sub| sub.try_map(standalone_merged_ins_to_syn))
                .collect()
        })?)),
        MergedInsNode::SingleIns(ins) => standalone_ins_to_syn(ins),
        _ => None,
    }
}

fn keep_only_ins_from_standalone_spine(spine: MergedSpineNode) -> Option<SynNode> {
    match spine {
        MergedSpineNode::Spine(spine) => Some(SynNode(
            spine.try_convert_into(keep_only_ins_from_standalone_spine_seq)?,
        )),
        MergedSpineNode::Unchanged => None,
        MergedSpineNode::Changed(_, ins) => standalone_merged_ins_to_syn(ins),
    }
}

fn keep_only_ins_from_standalone_spine_seq<'t>(
    seq: Vec<MergedSpineSeqNode<'t>>,
) -> Option<Vec<Subtree<SynNode<'t>>>> {
    seq.into_iter()
        .flat_map::<Box<dyn Iterator<Item = Option<Subtree<SynNode<'t>>>>>, _>(|node| match node {
            MergedSpineSeqNode::Zipped(spine) => Box::new(std::iter::once(
                spine.try_map(keep_only_ins_from_standalone_spine),
            )),
            MergedSpineSeqNode::Deleted(_) => Box::new(std::iter::empty()),
            MergedSpineSeqNode::Inserted(ins_list) => Box::new(
                ins_list
                    .into_iter()
                    .map(|sub| sub.try_map(standalone_ins_to_syn)),
            ),
            _ => Box::new(std::iter::once(None)),
        })
        .collect()
}

pub fn apply_patch<'t>(diff: MergedSpineNode<'t>, source: &SynNode<'t>) -> Option<SynNode<'t>> {
    let standalone_diff = remove_metavars(diff, source)?;
    keep_only_ins_from_standalone_spine(standalone_diff)
}
