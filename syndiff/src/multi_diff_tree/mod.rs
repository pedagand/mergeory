use crate::diff_tree::Metavariable;
use crate::family_traits::{Merge, VisitMut};

pub(crate) mod align_spine;
pub(crate) mod merge_del;
pub(crate) mod merge_ins;
pub(crate) mod metavar_renamer;
pub(crate) mod subst;
pub(crate) mod with_color;

pub use with_color::with_color;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color(pub u8);

#[derive(Clone)]
pub struct Colored<T> {
    pub node: T,
    pub colors: Vec<Color>,
}

impl<T> Colored<T> {
    pub fn new_white(node: T) -> Colored<T> {
        Colored {
            node,
            colors: vec![],
        }
    }
}

// Colors should always be merged together
impl<I1, I2, O, T> Merge<Colored<I1>, Colored<I2>, Colored<O>> for T
where
    T: Merge<I1, I2, O>,
{
    fn can_merge(&mut self, left: &Colored<I1>, right: &Colored<I2>) -> bool {
        self.can_merge(&left.node, &right.node)
    }

    fn merge(&mut self, left: Colored<I1>, right: Colored<I2>) -> Colored<O> {
        let mut colors = left.colors;
        colors.extend(right.colors);
        Colored {
            node: self.merge(left.node, right.node),
            colors,
        }
    }
}

#[derive(Clone)]
pub enum DelNode<D, I> {
    InPlace(D),
    Ellided(Metavariable),
    MetavariableConflict(Metavariable, Box<DelNode<D, I>>, InsNode<I>),
}

#[derive(Clone)]
pub enum InsNode<I> {
    InPlace(Colored<I>),
    Ellided(Colored<Metavariable>),
    Conflict(Vec<InsNode<I>>),
}
#[derive(Clone)]
pub enum InsSeqNode<I> {
    Node(InsNode<I>),
    DeleteConflict(InsNode<I>),
    InsertOrderConflict(Vec<Colored<Vec<InsNode<I>>>>),
}
#[derive(Clone)]
pub struct InsSeq<I>(pub Vec<InsSeqNode<I>>);

pub enum SpineNode<S, D, I> {
    Spine(S),
    Unchanged,
    Changed(DelNode<D, I>, InsNode<I>),
}
pub enum SpineSeqNode<S, D, I> {
    Zipped(SpineNode<S, D, I>),
    Deleted(Colored<DelNode<D, I>>),
    DeleteConflict(Colored<DelNode<D, I>>, InsNode<I>),
    Inserted(Colored<Vec<InsNode<I>>>),
    InsertOrderConflict(Vec<Colored<Vec<InsNode<I>>>>),
}
pub struct SpineSeq<S, D, I>(pub Vec<SpineSeqNode<S, D, I>>);

use crate::ast;
use align_spine::align_spine;
use merge_del::merge_del;
use merge_ins::merge_ins;
use metavar_renamer::MetavarRenamer;
use subst::Substituter;

pub fn merge_multi_diffs(
    left: ast::multi_diff::File,
    right: ast::multi_diff::File,
) -> Option<ast::multi_diff::File> {
    let (aligned, nb_metavars) = align_spine(left, right)?;
    let (ins_merged, ins_subst) = merge_ins(aligned, nb_metavars);
    let (mut merged, del_subst) = merge_del(ins_merged, &ins_subst)?;

    let mut subst = Substituter::new(del_subst, ins_subst);
    subst.visit_mut(&mut merged);

    let mut final_names = Vec::with_capacity(nb_metavars);
    let mut renamer = MetavarRenamer {
        new_metavars: &mut final_names,
        next_metavar: &mut 0,
    };
    renamer.visit_mut(&mut merged);

    Some(merged)
}
