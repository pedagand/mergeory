use super::{
    ColorSet, Colored, DelNode, InsNode, InsSeq, InsSeqNode, SpineNode, SpineSeq, SpineSeqNode,
};
use crate::diff_tree::{Aligned, AlignedSeq, ChangeNode, DiffNode};
use crate::family_traits::Convert;

pub struct WithColor(usize);

impl WithColor {
    fn color<T>(&self, node: T) -> Colored<T> {
        Colored {
            node,
            colors: ColorSet::from_color(self.0),
        }
    }
}

impl<I, O> Convert<I, Colored<O>> for WithColor
where
    WithColor: Convert<I, O>,
{
    fn convert(&mut self, input: I) -> Colored<O> {
        let converted = self.convert(input);
        self.color(converted)
    }
}

impl<C, I> Convert<ChangeNode<C>, InsNode<I>> for WithColor
where
    WithColor: Convert<C, Colored<I>>,
{
    fn convert(&mut self, input: ChangeNode<C>) -> InsNode<I> {
        match input {
            ChangeNode::InPlace(node) => InsNode::InPlace(self.convert(node)),
            ChangeNode::Ellided(mv) => InsNode::Ellided(self.color(mv)),
        }
    }
}

impl<C, I> Convert<Vec<ChangeNode<C>>, InsSeq<I>> for WithColor
where
    WithColor: Convert<ChangeNode<C>, InsNode<I>>,
{
    fn convert(&mut self, input: Vec<ChangeNode<C>>) -> InsSeq<I> {
        InsSeq(
            input
                .into_iter()
                .map(|node| InsSeqNode::Node(self.convert(node)))
                .collect(),
        )
    }
}

impl<C, D, I> Convert<ChangeNode<C>, DelNode<D, I>> for WithColor
where
    WithColor: Convert<C, Colored<D>>,
{
    fn convert(&mut self, input: ChangeNode<C>) -> DelNode<D, I> {
        match input {
            ChangeNode::InPlace(node) => DelNode::InPlace(self.convert(node)),
            ChangeNode::Ellided(mv) => DelNode::Ellided(self.color(mv)),
        }
    }
}

impl<S, MS, C, D, I> Convert<DiffNode<S, C>, SpineNode<MS, D, I>> for WithColor
where
    WithColor: Convert<S, MS>,
    WithColor: Convert<ChangeNode<C>, DelNode<D, I>>,
    WithColor: Convert<ChangeNode<C>, InsNode<I>>,
{
    fn convert(&mut self, input: DiffNode<S, C>) -> SpineNode<MS, D, I> {
        match input {
            DiffNode::Spine(spine) => SpineNode::Spine(self.convert(spine)),
            DiffNode::Changed(del, ins) => SpineNode::Changed(self.convert(del), self.convert(ins)),
            DiffNode::Unchanged(Some(mv)) => SpineNode::Changed(
                DelNode::Ellided(Colored::new_white(mv)),
                InsNode::Ellided(Colored::new_white(mv)),
            ),
            DiffNode::Unchanged(None) => SpineNode::Unchanged,
        }
    }
}

impl<S, MS, C, D, I> Convert<AlignedSeq<S, C>, SpineSeq<MS, D, I>> for WithColor
where
    WithColor: Convert<DiffNode<S, C>, SpineNode<MS, D, I>>,
    WithColor: Convert<ChangeNode<C>, DelNode<D, I>>,
    WithColor: Convert<ChangeNode<C>, InsNode<I>>,
{
    fn convert(&mut self, input: AlignedSeq<S, C>) -> SpineSeq<MS, D, I> {
        let mut seq = Vec::new();
        for node in input.0 {
            match node {
                Aligned::Zipped(spine) => seq.push(SpineSeqNode::Zipped(self.convert(spine))),
                Aligned::Deleted(del) => seq.push(SpineSeqNode::Deleted(self.convert(del))),
                Aligned::Inserted(ins) => {
                    // Squash together all successive inserts, to ease merge operations later
                    if let Some(SpineSeqNode::Inserted(ins_list)) = seq.last_mut() {
                        ins_list.node.push(self.convert(ins))
                    } else {
                        let ins = self.convert(ins);
                        seq.push(SpineSeqNode::Inserted(self.color(vec![ins])))
                    }
                }
            }
        }
        SpineSeq(seq)
    }
}

pub fn with_color<I, O>(input: I, color: usize) -> O
where
    WithColor: Convert<I, O>,
{
    WithColor(color).convert(input)
}
