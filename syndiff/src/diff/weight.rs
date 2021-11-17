use crate::generic_tree::{Subtree, Tree};
use crate::syn_tree::SynNode;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub type Weight = usize;
pub const NODE_WEIGHT: Weight = 0;
pub const LEAF_WEIGHT: Weight = 2;
pub const SPINE_LEAF_WEIGHT: Weight = 1;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct HashSum(u64);

pub struct WeightedNode<'t> {
    pub node: Tree<'t, Subtree<WeightedNode<'t>>>,
    pub hash: HashSum,
    pub weight: Weight,
}

impl<'t> Hash for WeightedNode<'t> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash.0)
    }
}

impl<'t> PartialEq for WeightedNode<'t> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}
impl<'t> Eq for WeightedNode<'t> {}

pub fn weight_tree<'t>(input: &SynNode<'t>) -> WeightedNode<'t> {
    let mut weight = match input.0 {
        Tree::Node(_, _) => NODE_WEIGHT,
        Tree::Leaf(_) => LEAF_WEIGHT,
    };
    let node = input.0.map_subtrees(|sub| {
        let hashed_sub = weight_tree(sub);
        weight += hashed_sub.weight;
        hashed_sub
    });
    let mut hasher = DefaultHasher::new();
    node.hash(&mut hasher);
    WeightedNode {
        node,
        hash: HashSum(hasher.finish()),
        weight,
    }
}
