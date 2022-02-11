use super::{
    compute_node_alignment, SeqNodeAlignment, SubtreeAlignmentAlgorithm, Weight, WeightedNode,
    SPINE_LEAF_WEIGHT,
};
use crate::generic_tree::Subtree;
use std::cmp::{max, Reverse};
use std::collections::BinaryHeap;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct AlignmentAStarNode {
    estimated_cost: Reverse<Weight>,
    source_edge: Option<SeqNodeAlignment>,
    cost: Weight,
    del_pos: usize,
    ins_pos: usize,
}

impl AlignmentAStarNode {
    fn new(
        cost: Weight,
        del_pos: usize,
        ins_pos: usize,
        source_edge: Option<SeqNodeAlignment>,
    ) -> Self {
        AlignmentAStarNode {
            estimated_cost: Reverse(cost + SPINE_LEAF_WEIGHT * max(del_pos, ins_pos)),
            source_edge,
            cost,
            del_pos,
            ins_pos,
        }
    }
}

pub(super) fn compute_minimal_alignment(
    del_seq: &[Subtree<WeightedNode>],
    ins_seq: &[Subtree<WeightedNode>],
    alignment: &mut Vec<SeqNodeAlignment>,
    sub_algorithm: SubtreeAlignmentAlgorithm,
) -> Weight {
    // Using an A* pathfinding approach:
    // Nodes are pair of position in both sequences, edges are edit operations, distance is cost.
    // Goal: arrive at position (0, 0) from (n, m).
    let mut visited_nodes = Vec::new();
    visited_nodes.resize_with((del_seq.len() + 1) * (ins_seq.len() + 1), Default::default);
    let node_index = |del_pos, ins_pos| del_pos + ins_pos * (del_seq.len() + 1);

    let mut to_visit_heap = BinaryHeap::new();
    to_visit_heap.push(AlignmentAStarNode::new(
        0,
        del_seq.len(),
        ins_seq.len(),
        None,
    ));

    let cost;
    loop {
        let node = to_visit_heap.pop().unwrap();
        match &mut visited_nodes[node_index(node.del_pos, node.ins_pos)] {
            Some(_) => continue,
            visit_node => *visit_node = Some(node.source_edge),
        };
        if node.del_pos == 0 && node.ins_pos == 0 {
            cost = node.cost;
            break;
        }

        if node.del_pos > 0 {
            to_visit_heap.push(AlignmentAStarNode::new(
                node.cost + del_seq[node.del_pos - 1].node.weight,
                node.del_pos - 1,
                node.ins_pos,
                Some(SeqNodeAlignment::Delete),
            ));
        }
        if node.ins_pos > 0 {
            to_visit_heap.push(AlignmentAStarNode::new(
                node.cost + ins_seq[node.ins_pos - 1].node.weight,
                node.del_pos,
                node.ins_pos - 1,
                Some(SeqNodeAlignment::Insert),
            ));
        }
        if node.del_pos > 0 && node.ins_pos > 0 {
            let del = &del_seq[node.del_pos - 1];
            let ins = &ins_seq[node.ins_pos - 1];
            if del.field == ins.field {
                let (cost, align) = compute_node_alignment(&del.node, &ins.node, sub_algorithm);
                to_visit_heap.push(AlignmentAStarNode::new(
                    node.cost + cost,
                    node.del_pos - 1,
                    node.ins_pos - 1,
                    Some(SeqNodeAlignment::Zip(align)),
                ));
            }
        }
    }

    // Reconstruct the edit sequence from visited nodes
    let mut cur_coord = (0, 0);
    while let Some(align_op) = visited_nodes[node_index(cur_coord.0, cur_coord.1)]
        .take()
        .unwrap()
    {
        cur_coord = match &align_op {
            SeqNodeAlignment::Zip(_) => (cur_coord.0 + 1, cur_coord.1 + 1),
            SeqNodeAlignment::Delete => (cur_coord.0 + 1, cur_coord.1),
            SeqNodeAlignment::Insert => (cur_coord.0, cur_coord.1 + 1),
        };
        alignment.push(align_op)
    }
    cost
}

pub const MINIMAL_ALIGNMENT: SubtreeAlignmentAlgorithm =
    SubtreeAlignmentAlgorithm(|del_seq, ins_seq, alignment| {
        compute_minimal_alignment(del_seq, ins_seq, alignment, MINIMAL_ALIGNMENT)
    });
