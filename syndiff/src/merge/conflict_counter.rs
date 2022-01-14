use super::{DelNode, MergedInsNode, MergedSpineNode, MergedSpineSeqNode};

fn count_conflicts_in_del_node(node: &DelNode, counter: &mut usize) {
    match node {
        DelNode::InPlace(del) => del
            .data
            .visit(|ch| count_conflicts_in_del_node(&ch.node, counter)),
        DelNode::Elided(_) => (),
        DelNode::MetavariableConflict(_, del, _) => {
            *counter += 1;
            count_conflicts_in_del_node(del, counter);
        }
    }
}

fn count_conflicts_in_merged_ins_node(node: &MergedInsNode, counter: &mut usize) {
    match node {
        MergedInsNode::InPlace(ins) => ins
            .data
            .visit(|ch| count_conflicts_in_merged_ins_node(&ch.node, counter)),
        MergedInsNode::Elided(_) => (),
        MergedInsNode::Conflict(..) => {
            *counter += 1;
        }
    }
}

fn count_conflicts_in_spine_node(node: &MergedSpineNode, counter: &mut usize) {
    match node {
        MergedSpineNode::Spine(spine) => {
            spine.visit(|ch| count_conflicts_in_spine_seq_node(ch, counter))
        }
        MergedSpineNode::Unchanged => (),
        MergedSpineNode::Changed(del, ins) => {
            count_conflicts_in_del_node(del, counter);
            count_conflicts_in_merged_ins_node(ins, counter);
        }
    }
}

fn count_conflicts_in_spine_seq_node(node: &MergedSpineSeqNode, counter: &mut usize) {
    match node {
        MergedSpineSeqNode::Zipped(spine) => count_conflicts_in_spine_node(&spine.node, counter),
        MergedSpineSeqNode::Deleted(del_list) => {
            for del in del_list {
                count_conflicts_in_del_node(&del.node, counter)
            }
        }
        MergedSpineSeqNode::DeleteConflict(_, del, _) => {
            *counter += 1;
            count_conflicts_in_del_node(del, counter);
        }
        MergedSpineSeqNode::Inserted(_) => (),
        MergedSpineSeqNode::InsertOrderConflict(..) => {
            *counter += 1;
        }
    }
}

pub fn count_conflicts(tree: &MergedSpineNode) -> usize {
    let mut counter = 0;
    count_conflicts_in_spine_node(tree, &mut counter);
    counter
}
