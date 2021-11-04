use crate::generic_tree::{Subtree, Tree};
use crate::syn_tree::SynNode;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub type Weight = usize;
pub const NODE_WEIGHT: Weight = 0;
pub const LEAF_WEIGHT: Weight = 2;
pub const ELISION_WEIGHT: Weight = 3;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct HashSum(u64);

pub struct HashedNode<'t> {
    pub node: Tree<'t, Subtree<HashedNode<'t>>>,
    pub hash: HashSum,
    pub weight: Weight,
}

impl<'t> Hash for HashedNode<'t> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash.0)
    }
}

impl<'t> PartialEq for HashedNode<'t> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}
impl<'t> Eq for HashedNode<'t> {}

pub fn hash_tree<'t>(input: &SynNode<'t>) -> HashedNode<'t> {
    let mut weight = match input.0 {
        Tree::Node(_, _) => NODE_WEIGHT,
        Tree::Leaf(_) => LEAF_WEIGHT,
    };
    let node = input.0.map_subtrees(|sub| {
        let hashed_sub = hash_tree(sub);
        weight += hashed_sub.weight;
        hashed_sub
    });
    let mut hasher = DefaultHasher::new();
    node.hash(&mut hasher);
    HashedNode {
        node,
        hash: HashSum(hasher.finish()),
        weight,
    }
}
