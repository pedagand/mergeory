use crate::diff::{ChangeNode, DiffSpineNode, DiffSpineSeqNode};
use crate::generic_tree::{Subtree, Tree};
use crate::tree_formatter::{TreeFormattable, TreeFormatter};
use crate::Metavariable;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Left,
    Right,
    Both,
}

impl std::ops::BitOr for Color {
    type Output = Color;
    fn bitor(self, rhs: Color) -> Color {
        match (self, rhs) {
            (Color::White, other) | (other, Color::White) => other,
            (Color::Left, Color::Left) => Color::Left,
            (Color::Right, Color::Right) => Color::Right,
            _ => Color::Both,
        }
    }
}

impl std::ops::BitOrAssign for Color {
    fn bitor_assign(&mut self, rhs: Color) {
        *self = *self | rhs
    }
}

#[derive(Clone, Copy)]
pub struct Colored<T> {
    pub data: T,
    pub color: Color,
}

impl<T> Colored<T> {
    pub fn new_white(data: T) -> Colored<T> {
        Colored {
            data,
            color: Color::White,
        }
    }

    pub fn new_both(data: T) -> Colored<T> {
        Colored {
            data,
            color: Color::Both,
        }
    }

    pub fn as_ref(&self) -> Colored<&T> {
        Colored {
            data: &self.data,
            color: self.color,
        }
    }

    pub fn map<U>(self, map_fn: impl FnOnce(T) -> U) -> Colored<U> {
        Colored {
            data: map_fn(self.data),
            color: self.color,
        }
    }

    pub fn merge<L, R>(
        left: Colored<L>,
        right: Colored<R>,
        merge_fn: impl FnOnce(L, R) -> Option<T>,
    ) -> Option<Colored<T>> {
        Some(Colored {
            data: merge_fn(left.data, right.data)?,
            color: left.color | right.color,
        })
    }
}

impl<T: TreeFormattable> TreeFormattable for Colored<T> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        fmt.write_colored(self.color, |fmt| self.data.write_with(fmt))
    }
}

pub enum ColoredChangeNode<'t> {
    InPlace(Colored<Tree<'t, Subtree<ColoredChangeNode<'t>>>>),
    Elided(Colored<Metavariable>),
}

pub enum ColoredSpineNode<'t> {
    Spine(Tree<'t, ColoredSpineSeqNode<'t>>),
    Unchanged,
    Changed(ColoredChangeNode<'t>, ColoredChangeNode<'t>),
}

pub enum ColoredSpineSeqNode<'t> {
    Zipped(Subtree<ColoredSpineNode<'t>>),
    Deleted(Vec<Subtree<ColoredChangeNode<'t>>>),
    Inserted(Vec<Subtree<ColoredChangeNode<'t>>>),
}

impl<'t> ColoredChangeNode<'t> {
    fn with_color(tree: &ChangeNode<'t>, color: Color) -> Self {
        match tree {
            ChangeNode::InPlace(node) => ColoredChangeNode::InPlace(Colored {
                data: node.map_subtrees(|sub| ColoredChangeNode::with_color(sub, color)),
                color,
            }),
            ChangeNode::Elided(mv) => ColoredChangeNode::Elided(Colored { data: *mv, color }),
        }
    }
}

impl<'t> ColoredSpineNode<'t> {
    pub fn with_color(tree: &DiffSpineNode<'t>, color: Color) -> Self {
        match tree {
            DiffSpineNode::Spine(node) => ColoredSpineNode::Spine(
                node.map_children(|ch| ColoredSpineSeqNode::with_color(ch, color)),
            ),
            DiffSpineNode::Unchanged => ColoredSpineNode::Unchanged,
            DiffSpineNode::Changed(del, ins) => ColoredSpineNode::Changed(
                ColoredChangeNode::with_color(del, color),
                ColoredChangeNode::with_color(ins, color),
            ),
        }
    }
}

impl<'t> ColoredSpineSeqNode<'t> {
    fn with_color(node: &DiffSpineSeqNode<'t>, color: Color) -> Self {
        match node {
            DiffSpineSeqNode::Zipped(subtree) => ColoredSpineSeqNode::Zipped(
                subtree
                    .as_ref()
                    .map(|node| ColoredSpineNode::with_color(node, color)),
            ),
            DiffSpineSeqNode::Deleted(del_list) => ColoredSpineSeqNode::Deleted(
                del_list
                    .iter()
                    .map(|del_subtree| {
                        del_subtree
                            .as_ref()
                            .map(|del| ColoredChangeNode::with_color(del, color))
                    })
                    .collect(),
            ),
            DiffSpineSeqNode::Inserted(ins_list) => ColoredSpineSeqNode::Inserted(
                ins_list
                    .iter()
                    .map(|ins_subtree| {
                        ins_subtree
                            .as_ref()
                            .map(|ins| ColoredChangeNode::with_color(ins, color))
                    })
                    .collect(),
            ),
        }
    }
}
