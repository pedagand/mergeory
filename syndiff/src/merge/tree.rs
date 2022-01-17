use super::colors::{Color, Colored, ColoredChangeNode};
use crate::generic_tree::{FieldId, Subtree, Tree};
use crate::syn_tree::SynNode;
use crate::tree_formatter::{TreeFormattable, TreeFormatter};
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

pub type InsNode<'t> = ColoredChangeNode<'t>;

pub enum MergedInsNode<'t> {
    InPlace(Tree<'t, Subtree<MergedInsNode<'t>>>),
    Elided(Metavariable),
    SingleIns(InsNode<'t>),
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
    Inserted(Vec<Subtree<InsNode<'t>>>),
    InsertOrderConflict(Vec<Subtree<InsNode<'t>>>, Vec<Subtree<InsNode<'t>>>),
}

impl<'t> DelNode<'t> {
    pub fn from_syn(tree: &SynNode<'t>, color: Color) -> Self {
        DelNode::InPlace(Colored {
            data: tree.0.map_subtrees(|sub| DelNode::from_syn(sub, color)),
            color,
        })
    }
}

impl<'t> TreeFormattable for DelNode<'t> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            DelNode::InPlace(node) => node.write_with(fmt),
            DelNode::Elided(mv) => {
                fmt.write_colored(mv.color, |fmt| fmt.write_metavariable(mv.data))
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
    pub fn from_syn(tree: &SynNode<'t>) -> Self {
        InsNode::InPlace(Colored::new_white(tree.0.map_subtrees(InsNode::from_syn)))
    }
}

impl<'t> TreeFormattable for InsNode<'t> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            InsNode::InPlace(node) => node.write_with(fmt),
            InsNode::Elided(mv) => {
                fmt.write_colored(mv.color, |fmt| fmt.write_metavariable(mv.data))
            }
        }
    }
}

impl<'t> TreeFormattable for MergedInsNode<'t> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            MergedInsNode::InPlace(node) => node.write_with(fmt),
            MergedInsNode::Elided(mv) => fmt.write_metavariable(*mv),
            MergedInsNode::SingleIns(ins) => ins.write_with(fmt),
            MergedInsNode::Conflict(left_ins, right_ins) => fmt.write_ins_conflict(
                |fmt| left_ins.write_with(fmt),
                |fmt| right_ins.write_with(fmt),
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
}

impl<'t> TreeFormattable for MergedSpineNode<'t> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            MergedSpineNode::Spine(spine) => spine.write_with(fmt),
            MergedSpineNode::Unchanged => fmt.write_unchanged(),
            MergedSpineNode::Changed(del, ins) => {
                fmt.write_changed(|fmt| del.write_with(fmt), |fmt| ins.write_with(fmt))
            }
        }
    }
}

impl<'t> TreeFormattable for MergedSpineSeqNode<'t> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        match self {
            MergedSpineSeqNode::Zipped(spine) => spine.node.write_with(fmt),
            MergedSpineSeqNode::Deleted(del_list) => {
                fmt.write_deleted(|fmt| del_list.write_with(fmt))
            }
            MergedSpineSeqNode::DeleteConflict(_, del, ins) => {
                fmt.write_del_conflict(|fmt: &mut F| del.write_with(fmt), |fmt| ins.write_with(fmt))
            }
            MergedSpineSeqNode::Inserted(ins_list) => {
                fmt.write_inserted(|fmt| ins_list.write_with(fmt))
            }
            MergedSpineSeqNode::InsertOrderConflict(left_ins, right_ins) => fmt.write_ord_conflict(
                |fmt| left_ins.write_with(fmt),
                |fmt| right_ins.write_with(fmt),
            ),
        }
    }
}
