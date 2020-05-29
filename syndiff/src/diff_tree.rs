use crate::ellided_tree::MaybeEllided;
use crate::family_traits::{Convert, Visit};
use crate::hash_tree::HashSum;
use crate::spine_tree::{Aligned as HAligned, AlignedSeq as HAlignedSeq, DiffNode as HDiffNode};
use quote::TokenStreamExt;
use std::any::TypeId;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Metavariable(pub usize);

impl std::fmt::Display for Metavariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl quote::ToTokens for Metavariable {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let metavar_lit = proc_macro2::Literal::usize_unsuffixed(self.0);
        tokens.append(metavar_lit)
    }
}

pub enum ChangeNode<T> {
    InPlace(T),
    Ellided(Metavariable),
}

pub enum DiffNode<Spine, Change> {
    Spine(Spine),
    Changed(ChangeNode<Change>, ChangeNode<Change>),
    Unchanged(Option<Metavariable>),
}

pub enum Aligned<Spine, Change> {
    Zipped(DiffNode<Spine, Change>),
    Deleted(ChangeNode<Change>),
    Inserted(ChangeNode<Change>),
}
pub struct AlignedSeq<Spine, Change>(pub Vec<Aligned<Spine, Change>>);

pub struct MetavariableNamer {
    metavars: HashMap<(TypeId, HashSum), Metavariable>,
    next_id: usize,
}

impl<T: 'static> Visit<MaybeEllided<T>> for MetavariableNamer
where
    MetavariableNamer: Visit<T>,
{
    fn visit(&mut self, input: &MaybeEllided<T>) {
        match input {
            MaybeEllided::InPlace(node) => self.visit(node),
            MaybeEllided::Ellided(hash) => {
                let key = (TypeId::of::<T>(), *hash);
                if let Entry::Vacant(entry) = self.metavars.entry(key) {
                    entry.insert(Metavariable(self.next_id));
                    self.next_id += 1;
                }
            }
        }
    }
}

impl<S, C: 'static> Visit<HDiffNode<S, C>> for MetavariableNamer
where
    MetavariableNamer: Visit<S>,
    MetavariableNamer: Visit<C>,
{
    fn visit(&mut self, input: &HDiffNode<S, C>) {
        match input {
            HDiffNode::Spine(spine) => self.visit(spine),
            HDiffNode::Changed(del, ins) => {
                self.visit(del);
                self.visit(ins);
            }
            HDiffNode::Unchanged(_) => (),
        }
    }
}

impl<S, C: 'static> Visit<HAlignedSeq<S, C>> for MetavariableNamer
where
    MetavariableNamer: Visit<HDiffNode<S, C>>,
    MetavariableNamer: Visit<C>,
{
    fn visit(&mut self, input: &HAlignedSeq<S, C>) {
        for elt in &input.0 {
            match elt {
                HAligned::Zipped(spine) => self.visit(spine),
                HAligned::Deleted(del) => self.visit(del),
                HAligned::Inserted(ins) => self.visit(ins),
            }
        }
    }
}

impl<In: 'static, Out> Convert<MaybeEllided<In>, ChangeNode<Out>> for MetavariableNamer
where
    MetavariableNamer: Convert<In, Out>,
{
    fn convert(&mut self, input: MaybeEllided<In>) -> ChangeNode<Out> {
        match input {
            MaybeEllided::InPlace(node) => ChangeNode::InPlace(self.convert(node)),
            MaybeEllided::Ellided(hash) => {
                ChangeNode::Ellided(self.metavars[&(TypeId::of::<In>(), hash)])
            }
        }
    }
}

impl<InS, InC: 'static, OutS, OutC> Convert<HDiffNode<InS, InC>, DiffNode<OutS, OutC>>
    for MetavariableNamer
where
    MetavariableNamer: Convert<InS, OutS>,
    MetavariableNamer: Convert<InC, OutC>,
{
    fn convert(&mut self, input: HDiffNode<InS, InC>) -> DiffNode<OutS, OutC> {
        match input {
            HDiffNode::Spine(spine) => DiffNode::Spine(self.convert(spine)),
            HDiffNode::Changed(del, ins) => DiffNode::Changed(self.convert(del), self.convert(ins)),
            HDiffNode::Unchanged(hash) => {
                DiffNode::Unchanged(self.metavars.get(&(TypeId::of::<InC>(), hash)).copied())
            }
        }
    }
}

impl<InS, InC: 'static, OutS, OutC> Convert<HAlignedSeq<InS, InC>, AlignedSeq<OutS, OutC>>
    for MetavariableNamer
where
    MetavariableNamer: Convert<HDiffNode<InS, InC>, DiffNode<OutS, OutC>>,
    MetavariableNamer: Convert<InC, OutC>,
{
    fn convert(&mut self, input: HAlignedSeq<InS, InC>) -> AlignedSeq<OutS, OutC> {
        AlignedSeq(
            input
                .0
                .into_iter()
                .map(|elt| match elt {
                    HAligned::Zipped(spine) => Aligned::Zipped(self.convert(spine)),
                    HAligned::Deleted(del) => Aligned::Deleted(self.convert(del)),
                    HAligned::Inserted(ins) => Aligned::Inserted(self.convert(ins)),
                })
                .collect(),
        )
    }
}

pub fn name_metavariables<In, Out>(input: In) -> Out
where
    MetavariableNamer: Visit<In>,
    MetavariableNamer: Convert<In, Out>,
{
    let mut namer = MetavariableNamer {
        metavars: HashMap::new(),
        next_id: 0,
    };
    namer.visit(&input);
    namer.convert(input)
}
