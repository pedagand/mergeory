use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub type NodeKind = u16;
pub type FieldId = u16;

#[derive(Copy, Clone)]
pub struct Token<'t> {
    hash: u64,
    bytes: &'t [u8],
}

impl<'t> Token<'t> {
    pub fn new(bytes: &'t [u8]) -> Self {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        Token {
            hash: hasher.finish(),
            bytes,
        }
    }
}

impl<'t> Hash for Token<'t> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash)
    }
}

impl<'t> PartialEq for Token<'t> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}
impl<'t> Eq for Token<'t> {}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Tree<'t, T> {
    Node(NodeKind, Vec<T>),
    Leaf(Token<'t>),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Subtree<T> {
    pub field: Option<FieldId>,
    pub node: T,
}

impl<'t, T> Tree<'t, T> {
    pub fn visit(&self, mut visit_fn: impl FnMut(&T)) {
        match self {
            Tree::Node(_, children) => {
                for ch in children {
                    visit_fn(ch)
                }
            }
            Tree::Leaf(_) => (),
        }
    }

    pub fn visit_mut(&mut self, mut visit_fn: impl FnMut(&mut T)) {
        match self {
            Tree::Node(_, children) => {
                for ch in children {
                    visit_fn(ch)
                }
            }
            Tree::Leaf(_) => (),
        }
    }

    pub fn convert<U>(&self, conv_fn: impl FnOnce(&[T]) -> Vec<U>) -> Tree<'t, U> {
        match self {
            Tree::Node(kind, children) => Tree::Node(*kind, conv_fn(children)),
            Tree::Leaf(tok) => Tree::Leaf(*tok),
        }
    }

    pub fn map_children<U>(&self, mut conv_fn: impl FnMut(&T) -> U) -> Tree<'t, U> {
        self.convert(|children| children.iter().map(&mut conv_fn).collect())
    }

    pub fn convert_into<U>(self, conv_fn: impl FnOnce(Vec<T>) -> Vec<U>) -> Tree<'t, U> {
        match self {
            Tree::Node(kind, children) => Tree::Node(kind, conv_fn(children)),
            Tree::Leaf(tok) => Tree::Leaf(tok),
        }
    }

    pub fn try_convert_into<U>(
        self,
        conv_fn: impl FnOnce(Vec<T>) -> Option<Vec<U>>,
    ) -> Option<Tree<'t, U>> {
        Some(match self {
            Tree::Node(kind, children) => Tree::Node(kind, conv_fn(children)?),
            Tree::Leaf(tok) => Tree::Leaf(tok),
        })
    }

    pub fn map_children_into<U>(self, mut conv_fn: impl FnMut(T) -> U) -> Tree<'t, U> {
        self.convert_into(|children| children.into_iter().map(&mut conv_fn).collect())
    }

    pub fn compare<U>(
        left: &Tree<'t, T>,
        right: &Tree<'t, U>,
        compare_children_fn: impl FnOnce(&[T], &[U]) -> bool,
    ) -> bool {
        match (left, right) {
            (Tree::Node(lkind, lch), Tree::Node(rkind, rch)) if lkind == rkind => {
                compare_children_fn(lch, rch)
            }
            (Tree::Leaf(ltok), Tree::Leaf(rtok)) => ltok == rtok,
            _ => false,
        }
    }

    pub fn merge_into<L, R>(
        left: Tree<'t, L>,
        right: Tree<'t, R>,
        merge_child_fn: impl FnOnce(Vec<L>, Vec<R>) -> Option<Vec<T>>,
    ) -> Option<Self> {
        match (left, right) {
            (Tree::Node(lkind, lch), Tree::Node(rkind, rch)) if lkind == rkind => {
                Some(Tree::Node(lkind, merge_child_fn(lch, rch)?))
            }
            (Tree::Leaf(ltok), Tree::Leaf(rtok)) if ltok == rtok => Some(Tree::Leaf(ltok)),
            _ => None,
        }
    }

    pub fn merge_to<L, R>(
        left: &Tree<'t, L>,
        right: &Tree<'t, R>,
        merge_child_fn: impl FnOnce(&[L], &[R]) -> Option<Vec<T>>,
    ) -> Option<Self> {
        match (left, right) {
            (Tree::Node(lkind, lch), Tree::Node(rkind, rch)) if lkind == rkind => {
                Some(Tree::Node(*lkind, merge_child_fn(lch, rch)?))
            }
            (Tree::Leaf(ltok), Tree::Leaf(rtok)) if ltok == rtok => Some(Tree::Leaf(*ltok)),
            _ => None,
        }
    }

    pub fn split_into<L, R>(
        self,
        split_children_fn: impl FnOnce(Vec<T>) -> (Vec<L>, Vec<R>),
    ) -> (Tree<'t, L>, Tree<'t, R>) {
        match self {
            Tree::Node(kind, children) => {
                let (sub_left, sub_right) = split_children_fn(children);
                (Tree::Node(kind, sub_left), Tree::Node(kind, sub_right))
            }
            Tree::Leaf(tok) => (Tree::Leaf(tok), Tree::Leaf(tok)),
        }
    }

    pub fn write_to<O: std::io::Write>(
        &self,
        output: &mut O,
        mut write_child_fn: impl FnMut(&T, &mut O) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        match self {
            Tree::Node(_, children) => {
                for ch in children {
                    write_child_fn(ch, output)?
                }
                Ok(())
            }
            Tree::Leaf(tok) => output.write_all(tok.bytes),
        }
    }
}

impl<T> Subtree<T> {
    pub fn as_ref(&self) -> Subtree<&T> {
        Subtree {
            field: self.field,
            node: &self.node,
        }
    }

    pub fn map<U>(self, conv_fn: impl FnOnce(T) -> U) -> Subtree<U> {
        Subtree {
            field: self.field,
            node: conv_fn(self.node),
        }
    }

    pub fn try_map<U>(self, conv_fn: impl FnOnce(T) -> Option<U>) -> Option<Subtree<U>> {
        Some(Subtree {
            field: self.field,
            node: conv_fn(self.node)?,
        })
    }

    pub fn compare<U>(
        left: &Subtree<T>,
        right: &Subtree<U>,
        compare_fn: impl FnOnce(&T, &U) -> bool,
    ) -> bool {
        if left.field != right.field {
            return false;
        }
        compare_fn(&left.node, &right.node)
    }

    pub fn merge<L, R>(
        left: Subtree<L>,
        right: Subtree<R>,
        merge_fn: impl FnOnce(L, R) -> Option<T>,
    ) -> Option<Self> {
        if left.field != right.field {
            return None;
        }
        Some(Subtree {
            field: left.field,
            node: merge_fn(left.node, right.node)?,
        })
    }
}

impl<'t, T> Tree<'t, Subtree<T>> {
    pub fn map_subtrees<U>(&self, mut conv_fn: impl FnMut(&T) -> U) -> Tree<'t, Subtree<U>> {
        self.map_children(|child| child.as_ref().map(&mut conv_fn))
    }

    pub fn map_subtrees_into<U>(self, mut conv_fn: impl FnMut(T) -> U) -> Tree<'t, Subtree<U>> {
        self.map_children_into(|child| child.map(&mut conv_fn))
    }

    pub fn merge_subtrees_into<L, R>(
        left: Tree<'t, Subtree<L>>,
        right: Tree<'t, Subtree<R>>,
        mut merge_child_fn: impl FnMut(L, R) -> Option<T>,
    ) -> Option<Self> {
        Tree::merge_into(left, right, |left_seq, right_seq| {
            if left_seq.len() != right_seq.len() {
                return None;
            }
            left_seq
                .into_iter()
                .zip(right_seq)
                .map(|(l, r)| Subtree::merge(l, r, &mut merge_child_fn))
                .collect()
        })
    }
}
