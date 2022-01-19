use super::metavar_remover::remove_metavars;
use super::{InsNode, MergedInsNode, MergedSpineNode, MergedSpineSeqNode};
use crate::generic_tree::Subtree;
use crate::SynNode;

fn try_collect_flat_map<S, I>(
    seq: S,
    mut convert_fn: impl FnMut(S::Item) -> Option<I>,
) -> Option<Vec<I::Item>>
where
    S: IntoIterator,
    I: IntoIterator,
{
    seq.into_iter().try_fold(Vec::new(), |mut acc, sub| {
        acc.extend(convert_fn(sub)?);
        Some(acc)
    })
}

fn try_broadcast_field<T, U>(
    sub: Subtree<T>,
    mut convert_fn: impl FnMut(T) -> Option<U>,
) -> Option<impl Iterator<Item = Subtree<U::Item>>>
where
    U: IntoIterator,
{
    Some(convert_fn(sub.node)?.into_iter().map(move |node| Subtree {
        field: sub.field,
        node,
    }))
}

fn standalone_ins_to_syn(node: InsNode) -> Option<Vec<SynNode>> {
    match node {
        InsNode::InPlace(ins) => Some(vec![SynNode(ins.data.try_convert_into(|ch| {
            try_collect_flat_map(ch, |sub| try_broadcast_field(sub, standalone_ins_to_syn))
        })?)]),
        InsNode::Elided(_) => None,
        InsNode::Inlined(repl) => try_collect_flat_map(repl.data, standalone_ins_to_syn),
    }
}

fn standalone_merged_ins_to_syn(node: MergedInsNode) -> Option<Vec<SynNode>> {
    match node {
        MergedInsNode::InPlace(ins) => Some(vec![SynNode(ins.try_convert_into(|ch| {
            try_collect_flat_map(ch, |sub| {
                try_broadcast_field(sub, standalone_merged_ins_to_syn)
            })
        })?)]),
        MergedInsNode::SingleIns(ins) => standalone_ins_to_syn(ins),
        MergedInsNode::Elided(_) | MergedInsNode::Conflict(..) => None,
    }
}

fn keep_only_ins_from_standalone_spine(spine: MergedSpineNode) -> Option<Vec<SynNode>> {
    match spine {
        MergedSpineNode::Spine(spine) => Some(vec![SynNode(
            spine.try_convert_into(keep_only_ins_from_standalone_spine_seq)?,
        )]),
        MergedSpineNode::Unchanged => None,
        MergedSpineNode::Changed(_, ins) => standalone_merged_ins_to_syn(ins),
    }
}

fn keep_only_ins_from_standalone_spine_seq<'t>(
    seq: Vec<MergedSpineSeqNode<'t>>,
) -> Option<Vec<Subtree<SynNode<'t>>>> {
    seq.into_iter()
        .try_fold(Vec::new(), |mut acc, sub| match sub {
            MergedSpineSeqNode::Zipped(spine) => {
                acc.extend(try_broadcast_field(
                    spine,
                    keep_only_ins_from_standalone_spine,
                )?);
                Some(acc)
            }
            MergedSpineSeqNode::Deleted(_) => Some(acc),
            MergedSpineSeqNode::Inserted(ins_list) => {
                for ins in ins_list {
                    acc.extend(try_broadcast_field(ins, standalone_ins_to_syn)?)
                }
                Some(acc)
            }
            MergedSpineSeqNode::DeleteConflict(..)
            | MergedSpineSeqNode::InsertOrderConflict(..) => None,
        })
}

pub fn apply_patch<'t>(diff: MergedSpineNode<'t>, source: &SynNode<'t>) -> Option<SynNode<'t>> {
    let standalone_diff = remove_metavars(diff, source)?;
    let spine_root_vec = keep_only_ins_from_standalone_spine(standalone_diff)?;
    if spine_root_vec.len() == 1 {
        Some(spine_root_vec.into_iter().next().unwrap())
    } else {
        None
    }
}
