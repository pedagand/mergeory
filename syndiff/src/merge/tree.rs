use crate::diff::ChangeNode;
use crate::generic_tree::{FieldId, Subtree, Tree};
use crate::syn_tree::SynNode;
use crate::tree_formatter::TreeFormatter;
use crate::{ColorSet, Colored, Metavariable};

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

pub type InsNode<'t> = ChangeNode<'t>;

pub enum MergedInsNode<'t> {
    InPlace(Colored<Tree<'t, Subtree<MergedInsNode<'t>>>>),
    Elided(Colored<Metavariable>),
    Conflict(InsNode<'t>, InsNode<'t>),
}

pub enum MergedSpineNode<'t> {
    Spine(Tree<'t, MergedSpineSeqNode<'t>>),
    Unchanged,
    Changed(DelNode<'t>, MergedInsNode<'t>),
}

pub enum MergedSpineSeqNode<'t> {
    Zipped(Subtree<MergedSpineNode<'t>>),
    Deleted(Vec<Subtree<DelNode<'t>>>),
    DeleteConflict(Option<FieldId>, DelNode<'t>, InsNode<'t>),
    Inserted(Colored<Vec<Subtree<InsNode<'t>>>>),
    InsertOrderConflict(
        Colored<Vec<Subtree<InsNode<'t>>>>,
        Colored<Vec<Subtree<InsNode<'t>>>>,
    ),
}

impl<'t> DelNode<'t> {
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

impl<'t> MergedInsNode<'t> {
    pub fn from_simple_ins(tree: InsNode<'t>) -> Self {
        match tree {
            InsNode::InPlace(node) => MergedInsNode::InPlace(
                node.map(|node| node.map_subtrees_into(Self::from_simple_ins)),
            ),
            InsNode::Elided(mv) => MergedInsNode::Elided(mv),
        }
    }

    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            MergedInsNode::InPlace(node) => fmt.write_colored(node.colors, |fmt| {
                node.data.write_with(fmt, |ch, fmt| ch.node.write_with(fmt))
            }),
            MergedInsNode::Elided(mv) => {
                fmt.write_colored(mv.colors, |fmt| fmt.write_metavariable(mv.data))
            }
            MergedInsNode::Conflict(left_ins, right_ins) => fmt.write_ins_conflict(
                [left_ins, right_ins]
                    .iter()
                    .map(|ins| |fmt: &mut F| ins.write_with(fmt)),
            ),
        }
    }
}

impl<'t> MergedSpineNode<'t> {
    pub fn from_syn(tree: &SynNode<'t>) -> Self {
        MergedSpineNode::Spine(tree.0.map_children(|sub| {
            MergedSpineSeqNode::Zipped(sub.as_ref().map(MergedSpineNode::from_syn))
        }))
    }

    pub fn write_with(&self, fmt: &mut impl TreeFormatter) -> std::io::Result<()> {
        match self {
            MergedSpineNode::Spine(spine) => spine.write_with(fmt, MergedSpineSeqNode::write_with),
            MergedSpineNode::Unchanged => fmt.write_unchanged(),
            MergedSpineNode::Changed(del, ins) => {
                fmt.write_changed(|fmt| del.write_with(fmt), |fmt| ins.write_with(fmt))
            }
        }
    }
}

impl<'t> MergedSpineSeqNode<'t> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            MergedSpineSeqNode::Zipped(spine) => spine.node.write_with(fmt),
            MergedSpineSeqNode::Deleted(del_list) => fmt.write_deleted(|fmt| {
                for del in del_list {
                    del.node.write_with(fmt)?;
                }
                Ok(())
            }),
            MergedSpineSeqNode::DeleteConflict(_, del, ins) => fmt
                .write_del_conflict(Some(|fmt: &mut F| del.write_with(fmt)), |fmt| {
                    ins.write_with(fmt)
                }),
            MergedSpineSeqNode::Inserted(ins_list) => fmt.write_inserted(|fmt| {
                fmt.write_colored(ins_list.colors, |fmt| {
                    for ins in &ins_list.data {
                        ins.node.write_with(fmt)?;
                    }
                    Ok(())
                })
            }),
            MergedSpineSeqNode::InsertOrderConflict(left_ins, right_ins) => {
                fmt.write_ord_conflict([left_ins, right_ins].iter().map(|ins_list| {
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
