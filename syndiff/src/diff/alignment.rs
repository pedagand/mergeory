use super::hash::{HashedNode, Weight};
use crate::generic_tree::{Subtree, Tree};

pub enum AlignedNode<'t> {
    Spine(Tree<'t, AlignedSeqNode<'t>>),
    Unchanged,
    Changed(HashedNode<'t>, HashedNode<'t>),
}

pub enum AlignedSeqNode<'t> {
    Zipped(Subtree<AlignedNode<'t>>),
    Deleted(Vec<Subtree<HashedNode<'t>>>),
    Inserted(Vec<Subtree<HashedNode<'t>>>),
}

enum NodeAlignment {
    Zip(Vec<SeqNodeAlignment>),
    Copy,
    Replace,
}

enum SeqNodeAlignment {
    Zip(NodeAlignment),
    Insert,
    Delete,
}

fn compute_node_alignment(del: &HashedNode, ins: &HashedNode) -> (Weight, NodeAlignment) {
    if del == ins {
        return (0, NodeAlignment::Copy);
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

fn compute_subtrees_alignment(
    del_seq: &[Subtree<HashedNode>],
    ins_seq: &[Subtree<HashedNode>],
) -> (Weight, Vec<SeqNodeAlignment>) {
    // Using a dynamic programming approach:
    // dyn_array[id][ii] = "Best cost for subproblem del_seq[0..id], ins_seq[0..ii]"
    let mut dyn_array = Vec::with_capacity(del_seq.len() + 1);

    // Fill first line with only insertions
    let mut first_row = Vec::with_capacity(ins_seq.len() + 1);
    first_row.push((0, None));
    for (ii, ins) in ins_seq.iter().enumerate() {
        let (prev_cost, _) = first_row[ii];
        first_row.push((prev_cost + ins.node.weight, Some(SeqNodeAlignment::Insert)));
    }
    dyn_array.push(first_row);

    for (id, del) in del_seq.iter().enumerate() {
        dyn_array.push(Vec::with_capacity(ins_seq.len() + 1));

        // First column has only deletions
        let (prev_cost, _) = dyn_array[id][0];
        dyn_array[id + 1].push((prev_cost + del.node.weight, Some(SeqNodeAlignment::Delete)));

        // All the rest must consider zipping, deletion and insertion
        for (ii, ins) in ins_seq.iter().enumerate() {
            // Compute the cost of insertion and deletion and remember
            // the best of the two
            let cost_after_insert = dyn_array[id + 1][ii].0 + ins.node.weight;
            let cost_after_delete = dyn_array[id][ii + 1].0 + del.node.weight;

            dyn_array[id + 1].push(if cost_after_delete <= cost_after_insert {
                (cost_after_delete, Some(SeqNodeAlignment::Delete))
            } else {
                (cost_after_insert, Some(SeqNodeAlignment::Insert))
            });

            // Try to zip if fields are the same
            if del.field == ins.field {
                let (cost, align) = compute_node_alignment(&del.node, &ins.node);
                let cost_after_zip = dyn_array[id][ii].0 + cost;

                // Keep zipping if it improves or maintain score
                if cost_after_zip <= dyn_array[id + 1][ii + 1].0 {
                    dyn_array[id + 1][ii + 1] = (cost_after_zip, Some(SeqNodeAlignment::Zip(align)))
                }
            }
        }
    }

    let cost = dyn_array[del_seq.len()][ins_seq.len()].0;

    let mut cur_coord = (del_seq.len(), ins_seq.len());
    let mut rev_alignment = Vec::new();
    while let Some(align_op) = dyn_array[cur_coord.0][cur_coord.1].1.take() {
        cur_coord = match &align_op {
            SeqNodeAlignment::Zip(_) => (cur_coord.0 - 1, cur_coord.1 - 1),
            SeqNodeAlignment::Delete => (cur_coord.0 - 1, cur_coord.1),
            SeqNodeAlignment::Insert => (cur_coord.0, cur_coord.1 - 1),
        };
        rev_alignment.push(align_op)
    }
    (cost, rev_alignment.into_iter().rev().collect())
}

fn align_nodes<'t>(
    del: HashedNode<'t>,
    ins: HashedNode<'t>,
    alignment: NodeAlignment,
) -> AlignedNode<'t> {
    match alignment {
        NodeAlignment::Zip(sub_align) => {
            if let (Tree::Node(_, sub_del), Tree::Node(kind, sub_ins)) = (del.node, ins.node) {
                AlignedNode::Spine(Tree::Node(
                    kind,
                    align_subtrees(sub_del, sub_ins, sub_align),
                ))
            } else {
                panic!("Wrong node alignment in align_nodes")
            }
        }
        NodeAlignment::Copy => match ins.node {
            Tree::Leaf(tok) => AlignedNode::Spine(Tree::Leaf(tok)),
            _ => AlignedNode::Unchanged,
        },
        NodeAlignment::Replace => AlignedNode::Changed(del, ins),
    }
}

fn align_subtrees<'t>(
    del: Vec<Subtree<HashedNode<'t>>>,
    ins: Vec<Subtree<HashedNode<'t>>>,
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

pub fn align_trees<'t>(del: HashedNode<'t>, ins: HashedNode<'t>) -> AlignedNode<'t> {
    let (_, align) = compute_node_alignment(&del, &ins);
    align_nodes(del, ins, align)
}
