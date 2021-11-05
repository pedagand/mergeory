use crate::generic_tree::{Subtree, Tree};

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
    fn write_to(&self, output: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            ChangeNode::InPlace(node) => node.write_to(output, |ch, out| ch.node.write_to(out)),
            ChangeNode::Elided(mv) => write!(output, "${}", mv.0),
        }
    }
}

impl<'t> SpineNode<'t> {
    pub fn write_to(&self, output: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            SpineNode::Spine(spine) => spine.write_to(output, SpineSeqNode::write_to),
            SpineNode::Unchanged => write!(output, "·"),
            SpineNode::Changed(del, ins) => {
                write!(output, "CHANGED![«")?;
                del.write_to(output)?;
                write!(output, "» -> «")?;
                ins.write_to(output)?;
                write!(output, "»]")
            }
        }
    }
}

impl<'t> SpineSeqNode<'t> {
    fn write_to(&self, output: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            SpineSeqNode::Zipped(node) => node.node.write_to(output),
            SpineSeqNode::Deleted(del_list) => {
                write!(output, "DELETED![")?;
                for del in del_list {
                    del.node.write_to(output)?;
                }
                write!(output, "]")
            }
            SpineSeqNode::Inserted(ins_list) => {
                write!(output, "INSERTED![")?;
                for ins in ins_list {
                    ins.node.write_to(output)?;
                }
                write!(output, "]")
            }
        }
    }
}
