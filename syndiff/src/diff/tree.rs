use crate::generic_tree::{Subtree, Tree, WriteTree};

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

impl<'t> WriteTree for ChangeNode<'t> {
    fn write_tree<O: std::io::Write>(&self, output: &mut O) -> std::io::Result<()> {
        match self {
            ChangeNode::InPlace(node) => node.write_tree(output),
            ChangeNode::Elided(mv) => write!(output, "${}", mv.0),
        }
    }
}

impl<'t> WriteTree for SpineNode<'t> {
    fn write_tree<O: std::io::Write>(&self, output: &mut O) -> std::io::Result<()> {
        match self {
            SpineNode::Spine(spine) => spine.write_tree(output),
            SpineNode::Unchanged => write!(output, "·"),
            SpineNode::Changed(del, ins) => {
                write!(output, "CHANGED![«")?;
                del.write_tree(output)?;
                write!(output, "» -> «")?;
                ins.write_tree(output)?;
                write!(output, "»]")
            }
        }
    }
}

impl<'t> WriteTree for SpineSeqNode<'t> {
    fn write_tree<O: std::io::Write>(&self, output: &mut O) -> std::io::Result<()> {
        match self {
            SpineSeqNode::Zipped(node) => node.write_tree(output),
            SpineSeqNode::Deleted(del_list) => {
                write!(output, "DELETED![")?;
                for del in del_list {
                    del.write_tree(output)?;
                }
                write!(output, "]")
            }
            SpineSeqNode::Inserted(ins_list) => {
                write!(output, "INSERTED![")?;
                for ins in ins_list {
                    ins.write_tree(output)?;
                }
                write!(output, "]")
            }
        }
    }
}
