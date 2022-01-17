use crate::generic_tree::{Subtree, Tree};
use crate::tree_formatter::{TreeFormattable, TreeFormatter};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Metavariable(pub usize);

pub enum ChangeNode<'t> {
    InPlace(Tree<'t, Subtree<ChangeNode<'t>>>),
    Elided(Metavariable),
}

pub enum DiffSpineNode<'t> {
    Spine(Tree<'t, DiffSpineSeqNode<'t>>),
    Unchanged,
    Changed(ChangeNode<'t>, ChangeNode<'t>),
}

pub enum DiffSpineSeqNode<'t> {
    Zipped(Subtree<DiffSpineNode<'t>>),
    Deleted(Vec<Subtree<ChangeNode<'t>>>),
    Inserted(Vec<Subtree<ChangeNode<'t>>>),
}

impl<'t> TreeFormattable for ChangeNode<'t> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            ChangeNode::InPlace(node) => node.write_with(fmt),
            ChangeNode::Elided(mv) => fmt.write_metavariable(*mv),
        }
    }
}

impl<'t> TreeFormattable for DiffSpineNode<'t> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            DiffSpineNode::Spine(spine) => spine.write_with(fmt),
            DiffSpineNode::Unchanged => fmt.write_unchanged(),
            DiffSpineNode::Changed(del, ins) => {
                fmt.write_changed(|fmt| del.write_with(fmt), |fmt| ins.write_with(fmt))
            }
        }
    }
}

impl<'t> TreeFormattable for DiffSpineSeqNode<'t> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            DiffSpineSeqNode::Zipped(node) => node.write_with(fmt),
            DiffSpineSeqNode::Deleted(del_list) => {
                fmt.write_deleted(|fmt| del_list.write_with(fmt))
            }
            DiffSpineSeqNode::Inserted(ins_list) => {
                fmt.write_inserted(|fmt| ins_list.write_with(fmt))
            }
        }
    }
}
