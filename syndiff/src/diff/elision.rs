use super::alignment::{AlignedNode, AlignedSeqNode};
use super::weight::{HashSum, WeightedNode, ELISION_WEIGHT};
use super::{ChangeNode, Metavariable, SpineNode, SpineSeqNode};
use crate::generic_tree::Tree;
use std::collections::{HashMap, HashSet};

fn collect_node_hashes(tree: &WeightedNode, hash_set: &mut HashSet<HashSum>) {
    if matches!(tree.node, Tree::Leaf(_)) {
        return; // Do not elide leaf nodes
    }
    hash_set.insert(tree.hash);
    tree.node
        .visit(|sub| collect_node_hashes(&sub.node, hash_set));
}

fn collect_change_node_hashes(
    tree: &AlignedNode,
    del_hash_set: &mut HashSet<HashSum>,
    ins_hash_set: &mut HashSet<HashSum>,
) {
    match tree {
        AlignedNode::Spine(spine) => {
            spine.visit(|sub| collect_changed_subtree_hashes(sub, del_hash_set, ins_hash_set))
        }
        AlignedNode::Unchanged => (),
        AlignedNode::Changed(del, ins) => {
            collect_node_hashes(del, del_hash_set);
            collect_node_hashes(ins, ins_hash_set);
        }
    }
}

fn collect_changed_subtree_hashes(
    subtree: &AlignedSeqNode,
    del_hash_set: &mut HashSet<HashSum>,
    ins_hash_set: &mut HashSet<HashSum>,
) {
    match subtree {
        AlignedSeqNode::Zipped(node) => {
            collect_change_node_hashes(&node.node, del_hash_set, ins_hash_set)
        }
        AlignedSeqNode::Deleted(del_list) => {
            for del in del_list {
                collect_node_hashes(&del.node, del_hash_set)
            }
        }
        AlignedSeqNode::Inserted(ins_list) => {
            for ins in ins_list {
                collect_node_hashes(&ins.node, ins_hash_set)
            }
        }
    }
}

fn collect_wanted_elisions(
    tree: &WeightedNode,
    possible_elisions: &HashSet<HashSum>,
    wanted_elisions: &mut HashSet<HashSum>,
) {
    if possible_elisions.contains(&tree.hash) {
        // Add the hash as a wanted elision and do NOT recurse
        wanted_elisions.insert(tree.hash);
    } else {
        tree.node
            .visit(|sub| collect_wanted_elisions(&sub.node, possible_elisions, wanted_elisions))
    }
}

fn collect_change_node_elisions(
    tree: &AlignedNode,
    possible_elisions: &HashSet<HashSum>,
    del_elisions: &mut HashSet<HashSum>,
    ins_elisions: &mut HashSet<HashSum>,
) {
    match tree {
        AlignedNode::Spine(spine) => spine.visit(|sub| {
            collect_changed_subtree_elisions(sub, possible_elisions, del_elisions, ins_elisions)
        }),
        AlignedNode::Unchanged => (),
        AlignedNode::Changed(del, ins) => {
            collect_wanted_elisions(del, possible_elisions, del_elisions);
            collect_wanted_elisions(ins, possible_elisions, ins_elisions);
        }
    }
}

fn collect_changed_subtree_elisions(
    subtree: &AlignedSeqNode,
    possible_elisions: &HashSet<HashSum>,
    del_elisions: &mut HashSet<HashSum>,
    ins_elisions: &mut HashSet<HashSum>,
) {
    match subtree {
        AlignedSeqNode::Zipped(node) => {
            collect_change_node_elisions(&node.node, possible_elisions, del_elisions, ins_elisions)
        }
        AlignedSeqNode::Deleted(del_list) => {
            for del in del_list {
                collect_wanted_elisions(&del.node, possible_elisions, del_elisions)
            }
        }
        AlignedSeqNode::Inserted(ins_list) => {
            for ins in ins_list {
                collect_wanted_elisions(&ins.node, possible_elisions, ins_elisions)
            }
        }
    }
}

fn find_wanted_elisions(tree: &AlignedNode) -> HashSet<HashSum> {
    // Find the common subtrees between deleted and inserted nodes
    let mut del_hashes = HashSet::new();
    let mut ins_hashes = HashSet::new();
    collect_change_node_hashes(tree, &mut del_hashes, &mut ins_hashes);
    let possible_elisions = &del_hashes & &ins_hashes;

    // Find which of the common subtrees will actually be elided in both trees.
    // This avoids elided part appearing only inside one of the subtrees.
    let mut del_elisions = HashSet::new();
    let mut ins_elisions = HashSet::new();
    collect_change_node_elisions(
        tree,
        &possible_elisions,
        &mut del_elisions,
        &mut ins_elisions,
    );
    &del_elisions & &ins_elisions
}

fn reduce_weight_for_hash(tree: &mut WeightedNode, elisions: &HashSet<HashSum>) {
    if elisions.contains(&tree.hash) {
        tree.weight = std::cmp::min(tree.weight, ELISION_WEIGHT);
    } else {
        tree.node
            .visit_mut(|sub| reduce_weight_for_hash(&mut sub.node, elisions));
    }
}

pub fn reduce_weight_on_elision_sites<'t>(
    del: WeightedNode<'t>,
    ins: WeightedNode<'t>,
) -> (WeightedNode<'t>, WeightedNode<'t>) {
    let aligned = AlignedNode::Changed(del, ins);
    let elisions = find_wanted_elisions(&aligned);
    let (mut del, mut ins) = match aligned {
        AlignedNode::Changed(del, ins) => (del, ins),
        _ => unreachable![],
    };
    reduce_weight_for_hash(&mut del, &elisions);
    reduce_weight_for_hash(&mut ins, &elisions);
    (del, ins)
}

#[derive(Default)]
struct MetavarNameGenerator {
    metavars: HashMap<HashSum, Metavariable>,
    next_id: usize,
}

impl MetavarNameGenerator {
    fn get(&mut self, hash: HashSum) -> Metavariable {
        let next_id = &mut self.next_id;
        *self.metavars.entry(hash).or_insert_with(|| {
            let id = *next_id;
            *next_id += 1;
            Metavariable(id)
        })
    }
}

fn elide_tree<'t>(
    tree: &WeightedNode<'t>,
    elisions: &HashSet<HashSum>,
    name_generator: &mut MetavarNameGenerator,
) -> ChangeNode<'t> {
    if elisions.contains(&tree.hash) {
        ChangeNode::Elided(name_generator.get(tree.hash))
    } else {
        ChangeNode::InPlace(
            tree.node
                .map_subtrees(|sub| elide_tree(sub, elisions, name_generator)),
        )
    }
}

fn elide_change_nodes<'t>(
    tree: &AlignedNode<'t>,
    elisions: &HashSet<HashSum>,
    name_generator: &mut MetavarNameGenerator,
) -> SpineNode<'t> {
    match tree {
        AlignedNode::Spine(spine) => SpineNode::Spine(
            spine.map_children(|sub| elide_changed_subtree(sub, elisions, name_generator)),
        ),
        AlignedNode::Unchanged => SpineNode::Unchanged,
        AlignedNode::Changed(del, ins) => SpineNode::Changed(
            elide_tree(del, elisions, name_generator),
            elide_tree(ins, elisions, name_generator),
        ),
    }
}

fn elide_changed_subtree<'t>(
    subtree: &AlignedSeqNode<'t>,
    elisions: &HashSet<HashSum>,
    name_generator: &mut MetavarNameGenerator,
) -> SpineSeqNode<'t> {
    match subtree {
        AlignedSeqNode::Zipped(node) => SpineSeqNode::Zipped(
            node.as_ref()
                .map(|node| elide_change_nodes(node, elisions, name_generator)),
        ),
        AlignedSeqNode::Deleted(del_list) => SpineSeqNode::Deleted(
            del_list
                .iter()
                .map(|del| {
                    del.as_ref()
                        .map(|del| elide_tree(del, elisions, name_generator))
                })
                .collect(),
        ),
        AlignedSeqNode::Inserted(ins_list) => SpineSeqNode::Inserted(
            ins_list
                .iter()
                .map(|ins| {
                    ins.as_ref()
                        .map(|ins| elide_tree(ins, elisions, name_generator))
                })
                .collect(),
        ),
    }
}

pub fn find_metavariable_elisions<'t>(
    tree: &AlignedNode<'t>,
    skip_elisions: bool,
) -> SpineNode<'t> {
    let elisions = if !skip_elisions {
        find_wanted_elisions(tree)
    } else {
        HashSet::new()
    };
    let mut name_generator = MetavarNameGenerator::default();
    elide_change_nodes(tree, &elisions, &mut name_generator)
}
