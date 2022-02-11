use super::minimal::compute_minimal_alignment;
use super::{
    NodeAlignment, SeqNodeAlignment, SubtreeAlignmentAlgorithm, Weight, WeightedNode,
    SPINE_LEAF_WEIGHT,
};
use crate::generic_tree::Subtree;
use std::cmp::min;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy)]
struct IdenticalNode {
    del_pos: usize,
    ins_pos: usize,
}

fn heaviest_common_subseq<S>(seq: S) -> Vec<IdenticalNode>
where
    S: IntoIterator<Item = (IdenticalNode, Weight)>,
{
    let mut set = BTreeMap::new();
    let mut pred = Vec::new();
    for (IdenticalNode { del_pos, ins_pos }, pure_weight) in seq {
        let (prev_weight, prev_ins_pos) = set
            .range(..del_pos)
            .next_back()
            .map(|(_, &(w, i))| (w, Some(i)))
            .unwrap_or((0, None));
        let cur_weight = prev_weight + pure_weight;
        let mut next_pair = set.range(del_pos..).next();
        while let Some((&next_val, &(next_weight, _))) = next_pair {
            if cur_weight < next_weight {
                break;
            }
            set.remove(&next_val);
            next_pair = set.range(del_pos..).next()
        }
        if next_pair.map(|(v, _)| del_pos < *v).unwrap_or(true) {
            set.insert(del_pos, (cur_weight, ins_pos));
        }
        pred.push((IdenticalNode { del_pos, ins_pos }, prev_ins_pos));
    }
    let mut final_pos = set
        .values()
        .next_back()
        .map(|&(_, i)| Some(i))
        .unwrap_or(None);
    let mut rev_subseq = vec![];
    while let Some(i) = final_pos {
        let (node, prev) = pred[pred.binary_search_by_key(&i, |(n, _)| n.ins_pos).unwrap()];
        rev_subseq.push(node);
        final_pos = prev;
    }
    rev_subseq
}

fn compute_unique_subtrees_alignment(
    del_seq: &[Subtree<WeightedNode>],
    ins_seq: &[Subtree<WeightedNode>],
    alignment: &mut Vec<SeqNodeAlignment>,
) -> Weight {
    // Find unique nodes in del and remember their position
    let mut unique_del_pos = HashMap::new();
    for (i, del) in del_seq.iter().enumerate() {
        match unique_del_pos.entry(del) {
            Entry::Occupied(entry) => *entry.into_mut() = usize::MAX,
            Entry::Vacant(entry) => {
                entry.insert(i);
            }
        }
    }
    unique_del_pos.retain(|_, v| *v != usize::MAX);

    // Find unique nodes in ins
    let mut unique_ins = HashMap::new();
    for ins in ins_seq {
        match unique_ins.entry(ins) {
            Entry::Occupied(entry) => *entry.into_mut() = false,
            Entry::Vacant(entry) => {
                entry.insert(true);
            }
        }
    }
    unique_ins.retain(|_, unique| *unique);

    let reversed_his =
        heaviest_common_subseq(ins_seq.iter().enumerate().filter_map(|(ins_pos, ins)| {
            if !unique_ins.contains_key(ins) {
                return None;
            }
            unique_del_pos
                .get(ins)
                .map(|&del_pos| (IdenticalNode { del_pos, ins_pos }, ins.node.weight))
        }));

    drop(unique_del_pos);
    drop(unique_ins);

    if reversed_his.is_empty() {
        compute_minimal_alignment(del_seq, ins_seq, alignment, PATIENCE_ALIGNMENT)
    } else {
        let mut cost = 0;
        let mut del_pos = 0;
        let mut ins_pos = 0;
        for id_node in reversed_his.into_iter().rev() {
            cost += compute_patience_alignment(
                &del_seq[del_pos..id_node.del_pos],
                &ins_seq[ins_pos..id_node.ins_pos],
                alignment,
            );
            alignment.push(SeqNodeAlignment::Zip(NodeAlignment::Copy));
            cost += SPINE_LEAF_WEIGHT;
            del_pos = id_node.del_pos + 1;
            ins_pos = id_node.ins_pos + 1;
        }
        cost += compute_patience_alignment(&del_seq[del_pos..], &ins_seq[ins_pos..], alignment);
        cost
    }
}

fn compute_patience_alignment(
    del_seq: &[Subtree<WeightedNode>],
    ins_seq: &[Subtree<WeightedNode>],
    alignment: &mut Vec<SeqNodeAlignment>,
) -> Weight {
    // First strip identical head and tail
    let nb_id_head = del_seq
        .iter()
        .zip(ins_seq)
        .position(|(del, ins)| del != ins)
        .unwrap_or(min(del_seq.len(), ins_seq.len()));
    let del_seq = &del_seq[nb_id_head..];
    let ins_seq = &ins_seq[nb_id_head..];

    let nb_id_tail = del_seq
        .iter()
        .rev()
        .zip(ins_seq.iter().rev())
        .position(|(del, ins)| del != ins)
        .unwrap_or(min(del_seq.len(), ins_seq.len()));
    let del_seq = &del_seq[..del_seq.len() - nb_id_tail];
    let ins_seq = &ins_seq[..ins_seq.len() - nb_id_tail];

    for _ in 0..nb_id_head {
        alignment.push(SeqNodeAlignment::Zip(NodeAlignment::Copy));
    }
    let inner_cost = compute_unique_subtrees_alignment(del_seq, ins_seq, alignment);
    for _ in 0..nb_id_tail {
        alignment.push(SeqNodeAlignment::Zip(NodeAlignment::Copy));
    }

    inner_cost + (nb_id_head + nb_id_tail) * SPINE_LEAF_WEIGHT
}

pub const PATIENCE_ALIGNMENT: SubtreeAlignmentAlgorithm =
    SubtreeAlignmentAlgorithm(compute_patience_alignment);
