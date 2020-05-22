use crate::ellided_tree::MaybeEllided;
use crate::family_traits::{Convert, Merge};
use crate::hash_tree::HashSum;
use crate::weighted_tree::{
    forget_weight, AlignableSeq, ForgetWeight, ForgettableWeight, Weighted,
};
use std::collections::VecDeque;

pub enum DiffNode<Spine, Change> {
    Spine(Spine),
    Changed(MaybeEllided<Change>, MaybeEllided<Change>),
    Unchanged(HashSum),
}

enum NodeAlign {
    Zip(SpineZipper),
    Copy,
    Change,
}

pub enum Aligned<Spine, Change> {
    Zipped(DiffNode<Spine, Change>),
    Deleted(MaybeEllided<Change>),
    Inserted(MaybeEllided<Change>),
}
pub struct AlignedSeq<Spine, Change>(pub Vec<Aligned<Spine, Change>>);

type SeqAlign = Vec<SeqAlignOp>;
enum SeqAlignOp {
    Zip(SpineZipper),
    Delete,
    Insert,
}

#[derive(Default)]
pub struct SpineZipper {
    seq_alignments: VecDeque<SeqAlign>,
    node_alignments: VecDeque<NodeAlign>,
    cost: u32,
}

impl<In: ForgettableWeight, Out> Merge<Weighted<In>, Weighted<In>, DiffNode<Out, In::WithoutWeight>>
    for SpineZipper
where
    SpineZipper: Merge<In, In, Out>,
    ForgetWeight: Convert<Weighted<In>, MaybeEllided<In::WithoutWeight>>,
{
    fn can_merge(&mut self, del: &Weighted<In>, ins: &Weighted<In>) -> bool {
        // A diff node always succeed at merging two trees but we must compute
        // the cost of doing that and therefore remember how to align under
        // ourselves
        let mut sub_zipper = SpineZipper::default();
        let node_align = match (&del.node, &ins.node) {
            (MaybeEllided::InPlace(del), MaybeEllided::InPlace(ins))
                if sub_zipper.can_merge(del, ins) =>
            {
                self.cost += sub_zipper.cost;
                NodeAlign::Zip(sub_zipper)
            }
            (MaybeEllided::Ellided(hdel), MaybeEllided::Ellided(hins)) if hdel == hins => {
                NodeAlign::Copy
            }
            _ => {
                self.cost += del.weight + ins.weight;
                NodeAlign::Change
            }
        };
        self.node_alignments.push_back(node_align);
        true
    }

    fn merge(&mut self, del: Weighted<In>, ins: Weighted<In>) -> DiffNode<Out, In::WithoutWeight> {
        match self.node_alignments.pop_front().unwrap() {
            NodeAlign::Zip(mut sub_zipper) => {
                if let (MaybeEllided::InPlace(del), MaybeEllided::InPlace(ins)) =
                    (del.node, ins.node)
                {
                    DiffNode::Spine(sub_zipper.merge(del, ins))
                } else {
                    panic!("Wrong node alignment applied by SpineZipper")
                }
            }
            NodeAlign::Copy => {
                if let MaybeEllided::Ellided(h) = del.node {
                    DiffNode::Unchanged(h)
                } else {
                    panic!("Wrong node alignment applied by SpineZipper")
                }
            }
            NodeAlign::Change => {
                let del = forget_weight(del);
                let ins = forget_weight(ins);
                DiffNode::Changed(del, ins)
            }
        }
    }
}

impl<In: ForgettableWeight, Out>
    Merge<AlignableSeq<In>, AlignableSeq<In>, AlignedSeq<Out, In::WithoutWeight>> for SpineZipper
where
    SpineZipper: Merge<Weighted<In>, Weighted<In>, DiffNode<Out, In::WithoutWeight>>,
    ForgetWeight: Convert<Weighted<In>, MaybeEllided<In::WithoutWeight>>,
{
    fn can_merge(&mut self, del: &AlignableSeq<In>, ins: &AlignableSeq<In>) -> bool {
        // We need to decide the alignment here because the merge operation can
        // only be called on the definitive pairs as it consumes the input.
        let AlignableSeq(del) = del;
        let AlignableSeq(ins) = ins;

        // Using a dynamic programming approach:
        // dyn_array[id][ii] = "Best cost for subproblem del[0..id], ins[0..ii]"
        let mut dyn_array = Vec::with_capacity(del.len() + 1);

        // Fill first line with only insertions
        let mut first_row = Vec::with_capacity(ins.len() + 1);
        first_row.push((0, None));
        for ii in 0..ins.len() {
            let (prev_cost, _) = first_row[ii];
            first_row.push((prev_cost + ins[ii].weight, Some(SeqAlignOp::Insert)));
        }
        dyn_array.push(first_row);

        for id in 0..del.len() {
            dyn_array.push(Vec::with_capacity(ins.len() + 1));

            // First column has only deletions
            let (prev_cost, _) = dyn_array[id][0];
            dyn_array[id + 1].push((prev_cost + del[id].weight, Some(SeqAlignOp::Delete)));

            // All the rest must consider zipping, deletion and insertion
            for ii in 0..ins.len() {
                // Compute the cost of insertion and deletion and remember
                // the best of the two
                let cost_after_insert = dyn_array[id + 1][ii].0 + ins[ii].weight;
                let cost_after_delete = dyn_array[id][ii + 1].0 + del[id].weight;

                dyn_array[id + 1].push(if cost_after_delete <= cost_after_insert {
                    (cost_after_delete, Some(SeqAlignOp::Delete))
                } else {
                    (cost_after_insert, Some(SeqAlignOp::Insert))
                });

                // Try to zip
                let mut sub_zipper = SpineZipper::default();
                if sub_zipper.can_merge(&del[id], &ins[ii]) {
                    let cost_after_zip = dyn_array[id][ii].0 + sub_zipper.cost;

                    // Keep zipping if it improves or maintain score
                    if cost_after_zip <= dyn_array[id + 1][ii + 1].0 {
                        dyn_array[id + 1][ii + 1] =
                            (cost_after_zip, Some(SeqAlignOp::Zip(sub_zipper)))
                    }
                }
            }
        }

        self.cost += dyn_array[del.len()][ins.len()].0;

        let mut cur_coord = (del.len(), ins.len());
        let mut rev_alignment = Vec::new();
        while let Some(align_op) = dyn_array[cur_coord.0][cur_coord.1].1.take() {
            cur_coord = match &align_op {
                SeqAlignOp::Zip(_) => (cur_coord.0 - 1, cur_coord.1 - 1),
                SeqAlignOp::Delete => (cur_coord.0 - 1, cur_coord.1),
                SeqAlignOp::Insert => (cur_coord.0, cur_coord.1 - 1),
            };
            rev_alignment.push(align_op)
        }
        self.seq_alignments
            .push_back(rev_alignment.into_iter().rev().collect());

        // We can align the sequences so we can merge them
        true
    }

    fn merge(
        &mut self,
        del: AlignableSeq<In>,
        ins: AlignableSeq<In>,
    ) -> AlignedSeq<Out, In::WithoutWeight> {
        let self_alignment = self.seq_alignments.pop_front().unwrap();

        let mut del_iter = del.0.into_iter();
        let mut ins_iter = ins.0.into_iter();
        let aligned_vec = self_alignment
            .into_iter()
            .map(|align_op| match align_op {
                SeqAlignOp::Zip(mut sub_zipper) => {
                    let sub_del = del_iter.next().unwrap();
                    let sub_ins = ins_iter.next().unwrap();
                    Aligned::Zipped(sub_zipper.merge(sub_del, sub_ins))
                }
                SeqAlignOp::Delete => {
                    let del = forget_weight(del_iter.next().unwrap());
                    Aligned::Deleted(del)
                }
                SeqAlignOp::Insert => {
                    let ins = forget_weight(ins_iter.next().unwrap());
                    Aligned::Inserted(ins)
                }
            })
            .collect();

        // Checking that the alignment did not forget elements
        assert!(del_iter.next().is_none());
        assert!(ins_iter.next().is_none());

        AlignedSeq(aligned_vec)
    }
}

pub fn zip_spine<In, Out>(in1: In, in2: In) -> Option<Out>
where
    SpineZipper: Merge<In, In, Out>,
{
    let mut zipper = SpineZipper::default();
    if zipper.can_merge(&in1, &in2) {
        Some(zipper.merge(in1, in2))
    } else {
        None
    }
}
