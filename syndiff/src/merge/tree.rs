use super::{Color, ColorSet, Colored};
use crate::diff::{ChangeNode, SpineNode as DiffSpineNode, SpineSeqNode as DiffSpineSeqNode};
use crate::generic_tree::{FieldId, Subtree, Tree};
use crate::syn_tree::SynNode;
use crate::tree_formatter::TreeFormatter;
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
            data: tree.0.map_subtrees(|sub| DelNode::from_syn(sub, colors)),
            colors,
        })
    }

    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            DelNode::InPlace(node) => fmt.write_colored(node.colors, |fmt| {
                node.data.write_with(fmt, |ch, fmt| ch.node.write_with(fmt))
            }),
            DelNode::Elided(mv) => {
                fmt.write_colored(mv.colors, |fmt| fmt.write_metavariable(mv.data))
            }
            DelNode::MetavariableConflict(mv, del, repl) => fmt.write_mv_conflict(
                *mv,
                |fmt| del.write_with(fmt),
                match repl {
                    MetavarInsReplacement::InferFromDel => None,
                    MetavarInsReplacement::Inlined(ins) => Some(|fmt: &mut F| ins.write_with(fmt)),
                },
            ),
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

    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            InsNode::InPlace(node) => fmt.write_colored(node.colors, |fmt| {
                node.data.write_with(fmt, InsSeqNode::write_with)
            }),
            InsNode::Elided(mv) => fmt.write_metavariable(*mv),
            InsNode::Conflict(confl) => {
                fmt.write_ins_conflict(confl.iter().map(|ins| |fmt: &mut F| ins.write_with(fmt)))
            }
        }
    }
}

impl<'t> InsSeqNode<'t> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            InsSeqNode::Node(node) => node.node.write_with(fmt),
            InsSeqNode::DeleteConflict(node) => fmt
                .write_del_conflict(None::<fn(&mut F) -> std::io::Result<()>>, |fmt| {
                    node.node.write_with(fmt)
                }),
            InsSeqNode::InsertOrderConflict(confl) => {
                fmt.write_ord_conflict(confl.iter().map(|ins_list| {
                    |fmt: &mut F| {
                        fmt.write_colored(ins_list.colors, |fmt| {
                            for ins in &ins_list.data {
                                ins.node.write_with(fmt)?;
                            }
                            Ok(())
                        })
                    }
                }))
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

    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            SpineSeqNode::Zipped(spine) => spine.node.write_with(fmt),
            SpineSeqNode::Deleted(del_list) => fmt.write_deleted(|fmt| {
                for del in del_list {
                    del.node.write_with(fmt)?;
                }
                Ok(())
            }),
            SpineSeqNode::DeleteConflict(_, del, ins) => fmt
                .write_del_conflict(Some(|fmt: &mut F| del.write_with(fmt)), |fmt| {
                    ins.write_with(fmt)
                }),
            SpineSeqNode::Inserted(ins_list) => fmt.write_inserted(|fmt| {
                fmt.write_colored(ins_list.colors, |fmt| {
                    for ins in &ins_list.data {
                        ins.node.write_with(fmt)?;
                    }
                    Ok(())
                })
            }),
            SpineSeqNode::InsertOrderConflict(confl) => {
                fmt.write_ord_conflict(confl.iter().map(|ins_list| {
                    |fmt: &mut F| {
                        fmt.write_colored(ins_list.colors, |fmt| {
                            for ins in &ins_list.data {
                                ins.node.write_with(fmt)?;
                            }
                            Ok(())
                        })
                    }
                }))
            }
        }
    }
}
