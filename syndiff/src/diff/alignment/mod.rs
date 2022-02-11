use super::weight::{HashSum, Weight, WeightedNode, SPINE_LEAF_WEIGHT};
use crate::generic_tree::{Subtree, Tree};
use std::cmp::Ordering;

mod minimal;
mod patience;

#[derive(Clone, Copy)]
pub struct SubtreeAlignmentAlgorithm(
    fn(&[Subtree<WeightedNode>], &[Subtree<WeightedNode>], &mut Vec<SeqNodeAlignment>) -> Weight,
);

pub use minimal::MINIMAL_ALIGNMENT;
pub use patience::PATIENCE_ALIGNMENT;

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

fn compute_node_alignment(
    del: &WeightedNode,
    ins: &WeightedNode,
    align_subtree_algorithm: SubtreeAlignmentAlgorithm,
) -> (Weight, NodeAlignment) {
    if del == ins {
        return (SPINE_LEAF_WEIGHT, NodeAlignment::Copy);
    }
    match (&del.node, &ins.node) {
        (Tree::Node(del_kind, del_sub), Tree::Node(ins_kind, ins_sub)) if del_kind == ins_kind => {
            let mut sub_align = Vec::new();
            let cost = align_subtree_algorithm.0(del_sub, ins_sub, &mut sub_align);
            if cost < del.weight + ins.weight {
                (cost, NodeAlignment::Zip(sub_align))
            } else {
                (del.weight + ins.weight, NodeAlignment::Replace)
            }
        }
        _ => (del.weight + ins.weight, NodeAlignment::Replace),
    }
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

pub fn align_trees<'t>(
    del: WeightedNode<'t>,
    ins: WeightedNode<'t>,
    subtree_algorithm: SubtreeAlignmentAlgorithm,
) -> AlignedNode<'t> {
    let (_, align) = compute_node_alignment(&del, &ins, subtree_algorithm);
    align_nodes(del, ins, align)
}
