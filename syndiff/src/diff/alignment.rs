use super::weight::{HashSum, Weight, WeightedNode, SPINE_LEAF_WEIGHT};
use crate::generic_tree::{Subtree, Tree};
use std::cmp::{max, Ordering, Reverse};
use std::collections::BinaryHeap;

pub enum AlignedNode<'t> {
    Spine(Tree<'t, AlignedSeqNode<'t>>, HashSum, HashSum),
    Unchanged(WeightedNode<'t>),
    Changed(WeightedNode<'t>, WeightedNode<'t>),
}

pub enum AlignedSeqNode<'t> {
    Zipped(Subtree<AlignedNode<'t>>),
    Deleted(Vec<Subtree<WeightedNode<'t>>>),
    Inserted(Vec<Subtree<WeightedNode<'t>>>),
}

enum NodeAlignment {
    Zip(Vec<SeqNodeAlignment>),
    Copy,
    Replace,
}

enum SeqNodeAlignment {
    Zip(NodeAlignment),
    Delete,
    Insert,
}

impl PartialEq for SeqNodeAlignment {
    fn eq(&self, other: &SeqNodeAlignment) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}
impl Eq for SeqNodeAlignment {}

impl PartialOrd for SeqNodeAlignment {
    fn partial_cmp(&self, other: &SeqNodeAlignment) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SeqNodeAlignment {
    fn cmp(&self, other: &SeqNodeAlignment) -> Ordering {
        match (self, other) {
            (SeqNodeAlignment::Zip(_), SeqNodeAlignment::Zip(_)) => Ordering::Equal,
            (SeqNodeAlignment::Zip(_), _) => Ordering::Greater,
            (_, SeqNodeAlignment::Zip(_)) => Ordering::Less,
            (SeqNodeAlignment::Delete, SeqNodeAlignment::Delete) => Ordering::Equal,
            (SeqNodeAlignment::Delete, _) => Ordering::Greater,
            (_, SeqNodeAlignment::Delete) => Ordering::Less,
            (SeqNodeAlignment::Insert, SeqNodeAlignment::Insert) => Ordering::Equal,
        }
    }
}

fn compute_node_alignment(del: &WeightedNode, ins: &WeightedNode) -> (Weight, NodeAlignment) {
    if del == ins {
        return (SPINE_LEAF_WEIGHT, NodeAlignment::Copy);
    }
    match (&del.node, &ins.node) {
        (Tree::Node(del_kind, del_sub), Tree::Node(ins_kind, ins_sub)) if del_kind == ins_kind => {
            let (cost, sub_align) = compute_subtrees_alignment(del_sub, ins_sub);
            if cost < del.weight + ins.weight {
                (cost, NodeAlignment::Zip(sub_align))
            } else {
                (del.weight + ins.weight, NodeAlignment::Replace)
            }
        }
        _ => (del.weight + ins.weight, NodeAlignment::Replace),
    }
}

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

fn compute_subtrees_alignment(
    del_seq: &[Subtree<WeightedNode>],
    ins_seq: &[Subtree<WeightedNode>],
) -> (Weight, Vec<SeqNodeAlignment>) {
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
                let (cost, align) = compute_node_alignment(&del.node, &ins.node);
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
    let mut alignment = Vec::new();
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
    (cost, alignment)
}

fn align_nodes<'t>(
    del: WeightedNode<'t>,
    ins: WeightedNode<'t>,
    alignment: NodeAlignment,
) -> AlignedNode<'t> {
    match alignment {
        NodeAlignment::Zip(sub_align) => {
            if let (Tree::Node(_, sub_del), Tree::Node(kind, sub_ins)) = (del.node, ins.node) {
                AlignedNode::Spine(
                    Tree::Node(kind, align_subtrees(sub_del, sub_ins, sub_align)),
                    del.hash,
                    ins.hash,
                )
            } else {
                panic!("Wrong node alignment in align_nodes")
            }
        }
        NodeAlignment::Copy => AlignedNode::Unchanged(ins),
        NodeAlignment::Replace => AlignedNode::Changed(del, ins),
    }
}

fn align_subtrees<'t>(
    del: Vec<Subtree<WeightedNode<'t>>>,
    ins: Vec<Subtree<WeightedNode<'t>>>,
    alignment: Vec<SeqNodeAlignment>,
) -> Vec<AlignedSeqNode<'t>> {
    let mut del_iter = del.into_iter();
    let mut ins_iter = ins.into_iter();
    let mut aligned_vec = Vec::new();
    for align in alignment {
        match align {
            SeqNodeAlignment::Zip(sub_align) => {
                let del = del_iter.next().unwrap();
                let ins = ins_iter.next().unwrap();
                debug_assert!(del.field == ins.field);
                aligned_vec.push(AlignedSeqNode::Zipped(Subtree {
                    field: ins.field,
                    node: align_nodes(del.node, ins.node, sub_align),
                }))
            }
            SeqNodeAlignment::Delete => {
                let del = del_iter.next().unwrap();
                if let Some(AlignedSeqNode::Deleted(del_list)) = aligned_vec.last_mut() {
                    del_list.push(del);
                } else {
                    aligned_vec.push(AlignedSeqNode::Deleted(vec![del]));
                }
            }
            SeqNodeAlignment::Insert => {
                let ins = ins_iter.next().unwrap();
                if let Some(AlignedSeqNode::Inserted(ins_list)) = aligned_vec.last_mut() {
                    ins_list.push(ins);
                } else {
                    aligned_vec.push(AlignedSeqNode::Inserted(vec![ins]));
                }
            }
        }
    }

    // Checking that the alignment did not forget elements
    assert!(del_iter.next().is_none());
    assert!(ins_iter.next().is_none());

    aligned_vec
}

pub fn align_trees<'t>(del: WeightedNode<'t>, ins: WeightedNode<'t>) -> AlignedNode<'t> {
    let (_, align) = compute_node_alignment(&del, &ins);
    align_nodes(del, ins, align)
}
