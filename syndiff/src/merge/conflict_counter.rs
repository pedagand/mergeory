use super::{DelNode, InsNode, InsSeqNode, MetavarInsReplacement, SpineNode, SpineSeqNode};

fn count_conflicts_in_del_node(node: &DelNode, counter: &mut usize) {
    match node {
        DelNode::InPlace(del) => del
            .node
            .visit(|ch| count_conflicts_in_del_node(&ch.node, counter)),
        DelNode::Elided(_) => (),
        DelNode::MetavariableConflict(_, del, repl) => {
            *counter += 1;
            count_conflicts_in_del_node(del, counter);
            match repl {
                MetavarInsReplacement::InferFromDel => (),
                MetavarInsReplacement::Inlined(ins) => count_conflicts_in_ins_node(ins, counter),
            }
        }
    }
}

fn count_conflicts_in_ins_node(node: &InsNode, counter: &mut usize) {
    match node {
        InsNode::InPlace(ins) => ins
            .node
            .visit(|ch| count_conflicts_in_ins_seq_node(ch, counter)),
        InsNode::Elided(_) => (),
        InsNode::Conflict(ins_list) => {
            *counter += 1;
            for ins in ins_list {
                count_conflicts_in_ins_node(ins, counter)
            }
        }
    }
}

fn count_conflicts_in_ins_seq_node(node: &InsSeqNode, counter: &mut usize) {
    match node {
        InsSeqNode::Node(ins) => count_conflicts_in_ins_node(&ins.node, counter),
        InsSeqNode::DeleteConflict(ins) => {
            *counter += 1;
            count_conflicts_in_ins_node(&ins.node, counter);
        }
        InsSeqNode::InsertOrderConflict(ins_list) => {
            *counter += 1;
            for ins_set in ins_list {
                for ins in &ins_set.node {
                    count_conflicts_in_ins_node(&ins.node, counter)
                }
            }
        }
    }
}

fn count_conflicts_in_spine_node(node: &SpineNode, counter: &mut usize) {
    match node {
        SpineNode::Spine(spine) => spine.visit(|ch| count_conflicts_in_spine_seq_node(ch, counter)),
        SpineNode::Unchanged => (),
        SpineNode::Changed(del, ins) => {
            count_conflicts_in_del_node(del, counter);
            count_conflicts_in_ins_node(ins, counter);
        }
    }
}

fn count_conflicts_in_spine_seq_node(node: &SpineSeqNode, counter: &mut usize) {
    match node {
        SpineSeqNode::Zipped(spine) => count_conflicts_in_spine_node(&spine.node, counter),
        SpineSeqNode::Deleted(del_list) => {
            for del in del_list {
                count_conflicts_in_del_node(&del.node, counter)
            }
        }
        SpineSeqNode::DeleteConflict(_, del, ins) => {
            *counter += 1;
            count_conflicts_in_del_node(del, counter);
            count_conflicts_in_ins_node(ins, counter);
        }
        SpineSeqNode::Inserted(ins_list) => {
            for ins in &ins_list.node {
                count_conflicts_in_ins_node(&ins.node, counter);
            }
        }
        SpineSeqNode::InsertOrderConflict(ins_list) => {
            *counter += 1;
            for ins_set in ins_list {
                for ins in &ins_set.node {
                    count_conflicts_in_ins_node(&ins.node, counter)
                }
            }
        }
    }
}

pub fn count_conflicts(tree: &SpineNode) -> usize {
    let mut counter = 0;
    count_conflicts_in_spine_node(tree, &mut counter);
    counter
}
