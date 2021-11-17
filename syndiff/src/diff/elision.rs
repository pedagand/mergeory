use super::alignment::{AlignedNode, AlignedSeqNode};
use super::weight::{HashSum, WeightedNode};
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
        AlignedNode::Spine(spine, del_hash, ins_hash) => {
            del_hash_set.insert(*del_hash);
            ins_hash_set.insert(*ins_hash);
            spine.visit(|sub| collect_changed_subtree_hashes(sub, del_hash_set, ins_hash_set))
        }
        AlignedNode::Unchanged(_) => (),
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
    del_elisions: Option<&mut HashSet<HashSum>>,
    ins_elisions: Option<&mut HashSet<HashSum>>,
) {
    if del_elisions.is_none() && ins_elisions.is_none() {
        return;
    }
    match tree {
        AlignedNode::Spine(spine, del_hash, ins_hash) => {
            // Stop collecting in the deletion/insertion subtrees if we want an elision here
            let mut del_elisions = del_elisions.and_then(|elisions| {
                if possible_elisions.contains(del_hash) {
                    elisions.insert(*del_hash);
                    None
                } else {
                    Some(elisions)
                }
            });
            let mut ins_elisions = ins_elisions.and_then(|elisions| {
                if possible_elisions.contains(ins_hash) {
                    elisions.insert(*ins_hash);
                    None
                } else {
                    Some(elisions)
                }
            });
            spine.visit(|sub| {
                collect_changed_subtree_elisions(
                    sub,
                    possible_elisions,
                    match &mut del_elisions {
                        Some(el) => Some(&mut **el),
                        None => None,
                    },
                    match &mut ins_elisions {
                        Some(el) => Some(&mut **el),
                        None => None,
                    },
                )
            })
        }
        AlignedNode::Unchanged(_) => (),
        AlignedNode::Changed(del, ins) => {
            if let Some(elisions) = del_elisions {
                collect_wanted_elisions(del, possible_elisions, elisions);
            }
            if let Some(elisions) = ins_elisions {
                collect_wanted_elisions(ins, possible_elisions, elisions);
            }
        }
    }
}

fn collect_changed_subtree_elisions(
    subtree: &AlignedSeqNode,
    possible_elisions: &HashSet<HashSum>,
    del_elisions: Option<&mut HashSet<HashSum>>,
    ins_elisions: Option<&mut HashSet<HashSum>>,
) {
    match subtree {
        AlignedSeqNode::Zipped(node) => {
            collect_change_node_elisions(&node.node, possible_elisions, del_elisions, ins_elisions)
        }
        AlignedSeqNode::Deleted(del_list) => {
            if let Some(elisions) = del_elisions {
                for del in del_list {
                    collect_wanted_elisions(&del.node, possible_elisions, elisions)
                }
            }
        }
        AlignedSeqNode::Inserted(ins_list) => {
            if let Some(elisions) = ins_elisions {
                for ins in ins_list {
                    collect_wanted_elisions(&ins.node, possible_elisions, elisions)
                }
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
        Some(&mut del_elisions),
        Some(&mut ins_elisions),
    );
    &del_elisions & &ins_elisions
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

fn elide_and_keep_del<'t>(
    tree: &AlignedNode<'t>,
    elisions: &HashSet<HashSum>,
    name_generator: &mut MetavarNameGenerator,
) -> ChangeNode<'t> {
    match tree {
        AlignedNode::Spine(_, del_hash, _) if elisions.contains(del_hash) => {
            ChangeNode::Elided(name_generator.get(*del_hash))
        }
        AlignedNode::Spine(spine, _, _) => ChangeNode::InPlace(spine.convert(|sub| {
            let mut del_sub = Vec::new();
            for sub_node in sub {
                match sub_node {
                    AlignedSeqNode::Zipped(node) => del_sub.push(
                        node.as_ref()
                            .map(|node| elide_and_keep_del(node, elisions, name_generator)),
                    ),
                    AlignedSeqNode::Deleted(del_list) => {
                        for del in del_list {
                            del_sub.push(
                                del.as_ref()
                                    .map(|del| elide_tree(del, elisions, name_generator)),
                            )
                        }
                    }
                    AlignedSeqNode::Inserted(_) => (),
                }
            }
            del_sub
        })),
        AlignedNode::Unchanged(node) => elide_tree(node, elisions, name_generator),
        AlignedNode::Changed(del, _) => elide_tree(del, elisions, name_generator),
    }
}

fn elide_and_keep_ins<'t>(
    tree: &AlignedNode<'t>,
    elisions: &HashSet<HashSum>,
    name_generator: &mut MetavarNameGenerator,
) -> ChangeNode<'t> {
    match tree {
        AlignedNode::Spine(_, _, ins_hash) if elisions.contains(ins_hash) => {
            ChangeNode::Elided(name_generator.get(*ins_hash))
        }
        AlignedNode::Spine(spine, _, _) => ChangeNode::InPlace(spine.convert(|sub| {
            let mut ins_sub = Vec::new();
            for sub_node in sub {
                match sub_node {
                    AlignedSeqNode::Zipped(node) => ins_sub.push(
                        node.as_ref()
                            .map(|node| elide_and_keep_ins(node, elisions, name_generator)),
                    ),
                    AlignedSeqNode::Inserted(ins_list) => {
                        for ins in ins_list {
                            ins_sub.push(
                                ins.as_ref()
                                    .map(|ins| elide_tree(ins, elisions, name_generator)),
                            )
                        }
                    }
                    AlignedSeqNode::Deleted(_) => (),
                }
            }
            ins_sub
        })),
        AlignedNode::Unchanged(node) => elide_tree(node, elisions, name_generator),
        AlignedNode::Changed(_, ins) => elide_tree(ins, elisions, name_generator),
    }
}

fn elide_change_nodes<'t>(
    tree: &AlignedNode<'t>,
    elisions: &HashSet<HashSum>,
    name_generator: &mut MetavarNameGenerator,
) -> SpineNode<'t> {
    match tree {
        AlignedNode::Spine(spine, del_hash, ins_hash) => {
            if elisions.contains(del_hash) || elisions.contains(ins_hash) {
                SpineNode::Changed(
                    elide_and_keep_del(tree, elisions, name_generator),
                    elide_and_keep_ins(tree, elisions, name_generator),
                )
            } else {
                SpineNode::Spine(
                    spine.map_children(|sub| elide_changed_subtree(sub, elisions, name_generator)),
                )
            }
        }
        AlignedNode::Unchanged(node) => match node.node {
            Tree::Leaf(tok) => SpineNode::Spine(Tree::Leaf(tok)),
            _ => SpineNode::Unchanged,
        },
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
