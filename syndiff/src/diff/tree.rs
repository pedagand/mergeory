use crate::generic_tree::{Subtree, Tree};
use crate::tree_formatter::TreeFormatter;
use crate::{Colored, SynNode};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Metavariable(pub usize);

#[derive(Clone)]
pub enum ChangeNode<'t> {
    InPlace(Colored<Tree<'t, Subtree<ChangeNode<'t>>>>),
    Elided(Colored<Metavariable>),
}

pub enum DiffSpineNode<'t> {
    Spine(Tree<'t, DiffSpineSeqNode<'t>>),
    Unchanged,
    Changed(ChangeNode<'t>, ChangeNode<'t>),
}

pub enum DiffSpineSeqNode<'t> {
    Zipped(Subtree<DiffSpineNode<'t>>),
    Deleted(Vec<Subtree<ChangeNode<'t>>>),
    Inserted(Colored<Vec<Subtree<ChangeNode<'t>>>>),
}

impl<'t> ChangeNode<'t> {
    pub fn from_syn(tree: &SynNode<'t>) -> Self {
        ChangeNode::InPlace(Colored::new_white(
            tree.0.map_subtrees(ChangeNode::from_syn),
        ))
    }

    pub fn write_with(&self, fmt: &mut impl TreeFormatter) -> std::io::Result<()> {
        match self {
            ChangeNode::InPlace(node) => fmt.write_colored(node.colors, |fmt| {
                node.data.write_with(fmt, |ch, fmt| ch.node.write_with(fmt))
            }),
            ChangeNode::Elided(mv) => {
                fmt.write_colored(mv.colors, |fmt| fmt.write_metavariable(mv.data))
            }
        }
    }
}

impl<'t> DiffSpineNode<'t> {
    pub fn write_with(&self, fmt: &mut impl TreeFormatter) -> std::io::Result<()> {
        match self {
            DiffSpineNode::Spine(spine) => spine.write_with(fmt, DiffSpineSeqNode::write_with),
            DiffSpineNode::Unchanged => fmt.write_unchanged(),
            DiffSpineNode::Changed(del, ins) => {
                fmt.write_changed(|fmt| del.write_with(fmt), |fmt| ins.write_with(fmt))
            }
        }
    }
}

impl<'t> DiffSpineSeqNode<'t> {
    fn write_with(&self, fmt: &mut impl TreeFormatter) -> std::io::Result<()> {
        match self {
            DiffSpineSeqNode::Zipped(node) => node.node.write_with(fmt),
            DiffSpineSeqNode::Deleted(del_list) => fmt.write_deleted(|fmt| {
                for del in del_list {
                    del.node.write_with(fmt)?;
                }
                Ok(())
            }),
            DiffSpineSeqNode::Inserted(ins_list) => fmt.write_inserted(|fmt| {
                for ins in &ins_list.data {
                    ins.node.write_with(fmt)?;
                }
                Ok(())
            }),
        }
    }
}
