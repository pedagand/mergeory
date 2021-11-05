use super::{Color, ColorSet, Colored};
use crate::diff::{ChangeNode, SpineNode as DiffSpineNode, SpineSeqNode as DiffSpineSeqNode};
use crate::generic_tree::{FieldId, Subtree, Tree};
use crate::syn_tree::SynNode;
use crate::Metavariable;

#[derive(Clone)]
pub enum MetavarInsReplacement<'t> {
    InferFromDel,
    Inlined(InsNode<'t>),
}

#[derive(Clone)]
pub enum DelNode<'t> {
    InPlace(Colored<Tree<'t, Subtree<DelNode<'t>>>>),
    Elided(Colored<Metavariable>),
    MetavariableConflict(Metavariable, Box<DelNode<'t>>, MetavarInsReplacement<'t>),
}

#[derive(Clone)]
pub enum InsNode<'t> {
    InPlace(Colored<Tree<'t, InsSeqNode<'t>>>),
    Elided(Metavariable),
    Conflict(Vec<InsNode<'t>>),
}

#[derive(Clone)]
pub enum InsSeqNode<'t> {
    Node(Subtree<InsNode<'t>>),
    DeleteConflict(Subtree<InsNode<'t>>),
    InsertOrderConflict(Vec<Colored<Vec<Subtree<InsNode<'t>>>>>),
}

pub enum SpineNode<'t> {
    Spine(Tree<'t, SpineSeqNode<'t>>),
    Unchanged,
    Changed(DelNode<'t>, InsNode<'t>),
}

pub enum SpineSeqNode<'t> {
    Zipped(Subtree<SpineNode<'t>>),
    Deleted(Vec<Subtree<DelNode<'t>>>),
    DeleteConflict(Option<FieldId>, DelNode<'t>, InsNode<'t>),
    Inserted(Colored<Vec<Subtree<InsNode<'t>>>>),
    InsertOrderConflict(Vec<Colored<Vec<Subtree<InsNode<'t>>>>>),
}

impl<'t> DelNode<'t> {
    fn with_color(tree: &ChangeNode<'t>, color: Color) -> Self {
        match tree {
            ChangeNode::InPlace(node) => DelNode::InPlace(Colored::with_color(
                color,
                node.map_subtrees(|ch| DelNode::with_color(ch, color)),
            )),
            ChangeNode::Elided(mv) => DelNode::Elided(Colored::with_color(color, *mv)),
        }
    }

    pub fn from_syn(tree: &SynNode<'t>, colors: ColorSet) -> Self {
        DelNode::InPlace(Colored {
            node: tree.0.map_subtrees(|sub| DelNode::from_syn(sub, colors)),
            colors,
        })
    }

    fn write_to(&self, output: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            DelNode::InPlace(node) => node.node.write_to(output, |ch, out| ch.node.write_to(out)),
            DelNode::Elided(mv) => write!(output, "${}", mv.node.0),
            DelNode::MetavariableConflict(mv, del, repl) => {
                write!(output, "MV_CONFLICT![${}: «", mv.0)?;
                del.write_to(output)?;
                write!(output, "»")?;
                match repl {
                    MetavarInsReplacement::InferFromDel => (),
                    MetavarInsReplacement::Inlined(ins) => {
                        write!(output, " <- «")?;
                        ins.write_to(output)?;
                        write!(output, "»")?;
                    }
                }
                write!(output, "]")
            }
        }
    }
}

impl<'t> InsNode<'t> {
    fn with_color(tree: &ChangeNode<'t>, color: Color) -> Self {
        match tree {
            ChangeNode::InPlace(node) => InsNode::InPlace(Colored::with_color(
                color,
                node.map_children(|ch| {
                    InsSeqNode::Node(ch.as_ref().map(|ch| InsNode::with_color(ch, color)))
                }),
            )),
            ChangeNode::Elided(mv) => InsNode::Elided(*mv),
        }
    }

    pub fn from_syn(tree: &SynNode<'t>) -> Self {
        InsNode::InPlace(Colored::new_white(
            tree.0
                .map_children(|ch| InsSeqNode::Node(ch.as_ref().map(InsNode::from_syn))),
        ))
    }

    fn write_to(&self, output: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            InsNode::InPlace(node) => node.node.write_to(output, InsSeqNode::write_to),
            InsNode::Elided(mv) => write!(output, "${}", mv.0),
            InsNode::Conflict(confl) => {
                write!(output, "CONFLICT![")?;
                for (i, ins) in confl.iter().enumerate() {
                    if i == 0 {
                        write!(output, "«")?
                    } else {
                        write!(output, ", «")?
                    }
                    ins.write_to(output)?;
                    write!(output, "»")?;
                }
                write!(output, "]")
            }
        }
    }
}

impl<'t> InsSeqNode<'t> {
    fn write_to(&self, output: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            InsSeqNode::Node(node) => node.node.write_to(output),
            InsSeqNode::DeleteConflict(node) => {
                write!(output, "DELETE_CONFLICT![")?;
                node.node.write_to(output)?;
                write!(output, "]")
            }
            InsSeqNode::InsertOrderConflict(confl) => {
                write!(output, "INSERT_ORDER_CONFLICT![")?;
                for (i, ins_list) in confl.iter().enumerate() {
                    if i == 0 {
                        write!(output, "«")?
                    } else {
                        write!(output, ", «")?
                    }
                    for ins in &ins_list.node {
                        ins.node.write_to(output)?;
                    }
                    write!(output, "»")?;
                }
                write!(output, "]")
            }
        }
    }
}

impl<'t> SpineNode<'t> {
    pub fn with_color(tree: &DiffSpineNode<'t>, color: Color) -> Self {
        match tree {
            DiffSpineNode::Spine(node) => {
                SpineNode::Spine(node.map_children(|sub| SpineSeqNode::with_color(sub, color)))
            }
            DiffSpineNode::Unchanged => SpineNode::Unchanged,
            DiffSpineNode::Changed(del, ins) => SpineNode::Changed(
                DelNode::with_color(del, color),
                InsNode::with_color(ins, color),
            ),
        }
    }

    pub fn from_syn(tree: &SynNode<'t>) -> Self {
        SpineNode::Spine(
            tree.0
                .map_children(|sub| SpineSeqNode::Zipped(sub.as_ref().map(SpineNode::from_syn))),
        )
    }

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
    fn with_color(subtree: &DiffSpineSeqNode<'t>, color: Color) -> Self {
        match subtree {
            DiffSpineSeqNode::Zipped(tree) => {
                SpineSeqNode::Zipped(tree.as_ref().map(|tree| SpineNode::with_color(tree, color)))
            }
            DiffSpineSeqNode::Deleted(del_list) => SpineSeqNode::Deleted(
                del_list
                    .iter()
                    .map(|del| del.as_ref().map(|del| DelNode::with_color(del, color)))
                    .collect(),
            ),
            DiffSpineSeqNode::Inserted(ins_list) => SpineSeqNode::Inserted(Colored::with_color(
                color,
                ins_list
                    .iter()
                    .map(|ins| ins.as_ref().map(|ins| InsNode::with_color(ins, color)))
                    .collect(),
            )),
        }
    }

    fn write_to(&self, output: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            SpineSeqNode::Zipped(spine) => spine.node.write_to(output),
            SpineSeqNode::Deleted(del_list) => {
                write!(output, "DELETED![")?;
                for del in del_list {
                    del.node.write_to(output)?;
                }
                write!(output, "]")
            }
            SpineSeqNode::DeleteConflict(_, del, ins) => {
                write!(output, "DELETE_CONFLICT![«")?;
                del.write_to(output)?;
                write!(output, "» -/> «")?;
                ins.write_to(output)?;
                write!(output, "»]")
            }
            SpineSeqNode::Inserted(ins_list) => {
                write!(output, "INSERTED![")?;
                for ins in &ins_list.node {
                    ins.node.write_to(output)?;
                }
                write!(output, "]")
            }
            SpineSeqNode::InsertOrderConflict(confl) => {
                write!(output, "INSERT_ORDER_CONFLICT![")?;
                for (i, ins_list) in confl.iter().enumerate() {
                    if i == 0 {
                        write!(output, "«")?
                    } else {
                        write!(output, ", «")?
                    }
                    for ins in &ins_list.node {
                        ins.node.write_to(output)?;
                    }
                    write!(output, "»")?;
                }
                write!(output, "]")
            }
        }
    }
}
