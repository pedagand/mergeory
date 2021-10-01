use crate::diff_tree::Metavariable;
use crate::family_traits::{Merge, VisitMut};
use quote::quote;

pub(crate) mod align_spine;
pub(crate) mod conflict_counter;
pub(crate) mod id_merger;
pub(crate) mod merge_del;
pub(crate) mod merge_ins;
pub(crate) mod metavar_remover;
pub(crate) mod metavar_renamer;
pub(crate) mod subst;
pub(crate) mod with_color;

pub use conflict_counter::count_conflicts;
pub use metavar_remover::remove_metavars;
pub use metavar_renamer::canonicalize_metavars;
pub use with_color::with_color;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ColorSet(u64);

impl ColorSet {
    pub fn white() -> ColorSet {
        ColorSet(0)
    }

    pub fn from_color(color: usize) -> ColorSet {
        ColorSet(1 << color)
    }

    pub fn contains(&self, color: usize) -> bool {
        self.0 & (1 << color) != 0
    }

    pub fn color_list(&self) -> Vec<u8> {
        let mut list = Vec::new();
        for c in 0..64 {
            if self.contains(c as usize) {
                list.push(c)
            }
        }
        list
    }
}

impl std::ops::BitOr for ColorSet {
    type Output = ColorSet;
    fn bitor(self, rhs: ColorSet) -> ColorSet {
        ColorSet(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for ColorSet {
    fn bitor_assign(&mut self, rhs: ColorSet) {
        self.0 |= rhs.0
    }
}

impl quote::ToTokens for ColorSet {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        if *self == ColorSet::white() {
            tokens.extend(std::iter::once(quote!(_)));
            return;
        }

        let color_lit = self
            .color_list()
            .into_iter()
            .map(|c| proc_macro2::Literal::u8_unsuffixed(c));
        tokens.extend(std::iter::once(quote!(#(#color_lit)&*)))
    }
}

#[derive(Clone, Copy)]
pub struct Colored<T> {
    pub node: T,
    pub colors: ColorSet,
}

impl<T> Colored<T> {
    pub fn new_white(node: T) -> Colored<T> {
        Colored {
            node,
            colors: ColorSet::white(),
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
        Colored {
            node: self.merge(left.node, right.node),
            colors: left.colors | right.colors,
        }
    }
}

#[derive(Clone)]
pub enum DelNode<D, I> {
    InPlace(Colored<D>),
    Elided(Colored<Metavariable>),
    MetavariableConflict(Metavariable, Box<DelNode<D, I>>, InsNode<I>),
}

#[derive(Clone)]
pub enum InsNode<I> {
    InPlace(Colored<I>),
    Elided(Metavariable),
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
    Deleted(DelNode<D, I>),
    DeleteConflict(DelNode<D, I>, InsNode<I>),
    Inserted(Colored<Vec<InsNode<I>>>),
    InsertOrderConflict(Vec<Colored<Vec<InsNode<I>>>>),
}
pub struct SpineSeq<S, D, I>(pub Vec<SpineSeqNode<S, D, I>>);

use crate::ast;
use align_spine::align_spine;
use merge_del::merge_del;
use merge_ins::merge_ins;
use metavar_renamer::rename_metavars;
use subst::{SolvedConflictsRemover, Substituter};

pub fn merge_multi_diffs(
    mut left: ast::multi_diff::File,
    mut right: ast::multi_diff::File,
) -> Option<ast::multi_diff::File> {
    let left_end_mv = rename_metavars(&mut left, 0);
    let right_end_mv = rename_metavars(&mut right, left_end_mv);
    let (aligned, nb_metavars) = align_spine(left, right, right_end_mv)?;

    let (ins_merged, ins_subst) = merge_ins(aligned, nb_metavars);
    let (mut merged, del_subst) = merge_del(ins_merged, nb_metavars)?;

    let mut subst = Substituter::new(del_subst, ins_subst);
    subst.visit_mut(&mut merged);
    let mut conflict_remover = SolvedConflictsRemover(subst);
    conflict_remover.visit_mut(&mut merged);

    Some(merged)
}
