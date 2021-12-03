use crate::generic_tree::{Subtree, Tree};
use crate::tree_formatter::TreeFormatter;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Metavariable(pub usize);

pub enum ChangeNode<'t> {
    InPlace(Tree<'t, Subtree<ChangeNode<'t>>>),
    Elided(Metavariable),
}

pub enum SpineNode<'t> {
    Spine(Tree<'t, SpineSeqNode<'t>>),
    Unchanged,
    Changed(ChangeNode<'t>, ChangeNode<'t>),
}

pub enum SpineSeqNode<'t> {
    Zipped(Subtree<SpineNode<'t>>),
    Deleted(Vec<Subtree<ChangeNode<'t>>>),
    Inserted(Vec<Subtree<ChangeNode<'t>>>),
}

impl<'t> ChangeNode<'t> {
    fn write_with(&self, fmt: &mut impl TreeFormatter) -> std::io::Result<()> {
        match self {
            ChangeNode::InPlace(node) => node.write_with(fmt, |ch, fmt| ch.node.write_with(fmt)),
            ChangeNode::Elided(mv) => fmt.write_metavariable(*mv),
        }
    }
}

impl<'t> SpineNode<'t> {
    pub fn write_with(&self, fmt: &mut impl TreeFormatter) -> std::io::Result<()> {
        match self {
            SpineNode::Spine(spine) => spine.write_with(fmt, SpineSeqNode::write_with),
            SpineNode::Unchanged => fmt.write_unchanged(),
            SpineNode::Changed(del, ins) => {
                fmt.write_changed(|fmt| del.write_with(fmt), |fmt| ins.write_with(fmt))
            }
        }
    }
}

impl<'t> SpineSeqNode<'t> {
    fn write_with(&self, fmt: &mut impl TreeFormatter) -> std::io::Result<()> {
        match self {
            SpineSeqNode::Zipped(node) => node.node.write_with(fmt),
            SpineSeqNode::Deleted(del_list) => fmt.write_deleted(|fmt| {
                for del in del_list {
                    del.node.write_with(fmt)?;
                }
                Ok(())
            }),
            SpineSeqNode::Inserted(ins_list) => fmt.write_inserted(|fmt| {
                for ins in ins_list {
                    ins.node.write_with(fmt)?;
                }
                Ok(())
            }),
        }
    }
}
