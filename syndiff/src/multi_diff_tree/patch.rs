use super::metavar_remover::{remove_metavars, MetavarRemover};
use super::{InsNode, InsSeq, InsSeqNode, SpineNode, SpineSeq, SpineSeqNode};
use crate::ast;
use crate::ast::multi_diff::SynType;
use crate::family_traits::{Convert, Merge, VisitMut};
use crate::token_trees::TokenTree;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::str::FromStr;

pub struct InsProjection;

impl<I, O> Convert<InsNode<I>, O> for InsProjection
where
    InsProjection: Convert<I, O>,
    O: SynType,
{
    fn convert(&mut self, node: InsNode<I>) -> O {
        match node {
            InsNode::InPlace(i) => self.convert(i.node),
            InsNode::Elided(_) => panic!("Cannot apply patch: remaining metavariable"),
            InsNode::Conflict(_) => panic!("Cannot apply patch: conflict remaining"),
        }
    }
}

impl<I, O> Convert<InsSeqNode<I>, O> for InsProjection
where
    InsProjection: Convert<InsNode<I>, O>,
    O: SynType,
{
    fn convert(&mut self, node: InsSeqNode<I>) -> O {
        match node {
            InsSeqNode::Node(n) => self.convert(n),
            _ => panic!("Cannot apply patch: conflict remaining"),
        }
    }
}

impl<I, O> Convert<InsSeq<I>, Vec<O>> for InsProjection
where
    InsProjection: Convert<Vec<InsSeqNode<I>>, Vec<O>>,
{
    fn convert(&mut self, seq: InsSeq<I>) -> Vec<O> {
        self.convert(seq.0)
    }
}

impl<I> Convert<InsSeq<I>, TokenStream> for InsProjection
where
    InsProjection: Convert<Vec<InsSeqNode<I>>, TokenStream>,
{
    fn convert(&mut self, seq: InsSeq<I>) -> TokenStream {
        self.convert(seq.0)
    }
}

impl<S, D, I, O> Convert<SpineNode<S, D, I>, O> for InsProjection
where
    InsProjection: Convert<S, O>,
    InsProjection: Convert<InsNode<I>, O>,
    O: SynType,
{
    fn convert(&mut self, node: SpineNode<S, D, I>) -> O {
        match node {
            SpineNode::Spine(spine) => self.convert(spine),
            SpineNode::Unchanged => panic!("Cannot apply patch: remaining unchanged"),
            SpineNode::Changed(_, ins) => self.convert(ins),
        }
    }
}

impl<S, D, I, O> Convert<SpineSeqNode<S, D, I>, Vec<O>> for InsProjection
where
    InsProjection: Convert<SpineNode<S, D, I>, O>,
    InsProjection: Convert<InsNode<I>, O>,
{
    fn convert(&mut self, node: SpineSeqNode<S, D, I>) -> Vec<O> {
        match node {
            SpineSeqNode::Zipped(spine) => vec![self.convert(spine)],
            SpineSeqNode::Deleted(_) => Vec::new(),
            SpineSeqNode::Inserted(ins_list) => self.convert(ins_list.node),
            _ => panic!("Cannot apply patch: conflict remaining"),
        }
    }
}

impl<S, D, I, O> Convert<SpineSeq<S, D, I>, Vec<O>> for InsProjection
where
    InsProjection: Convert<SpineSeqNode<S, D, I>, Vec<O>>,
{
    fn convert(&mut self, seq: SpineSeq<S, D, I>) -> Vec<O> {
        seq.0
            .into_iter()
            .flat_map(|node| self.convert(node))
            .collect()
    }
}

impl<S, D, I> Convert<SpineSeq<S, D, I>, TokenStream> for InsProjection
where
    InsProjection: Convert<SpineSeqNode<S, D, I>, Vec<TokenTree>>,
{
    fn convert(&mut self, seq: SpineSeq<S, D, I>) -> TokenStream {
        seq.0
            .into_iter()
            .flat_map(|node| self.convert(node))
            .flat_map(TokenTree::into_token_stream)
            .collect()
    }
}

// Miscellaneous implementations to expand ignored stuff

impl Convert<String, proc_macro2::Literal> for InsProjection {
    fn convert(&mut self, input: String) -> proc_macro2::Literal {
        match TokenStream::from_str(&input).unwrap().into_iter().next() {
            Some(proc_macro2::TokenTree::Literal(lit)) => lit,
            _ => panic!("Non literal token found at literal position"),
        }
    }
}

impl Convert<(), Span> for InsProjection {
    fn convert(&mut self, _: ()) -> Span {
        Span::call_site()
    }
}

// Workaround to be able to recreate the raw field of private type Reserved
macro_rules! convert_expr_reference {
    { $($in_typ:ty,)* } => {
        $(impl Convert<$in_typ, syn::ExprReference> for InsProjection {
            fn convert(&mut self, input: $in_typ) -> syn::ExprReference {
                syn::ExprReference {
                    attrs: self.convert(input.attrs),
                    and_token: input.and_token,
                    raw: Default::default(),
                    mutability: input.mutability,
                    expr: self.convert(input.expr),
                }
            }
        })*
    }
}
convert_expr_reference!(
    ast::multi_diff::ins::ExprReference,
    ast::multi_diff::ExprReference,
);

pub fn apply_patch<P, T>(multi_diff: P, source: T) -> Option<T>
where
    MetavarRemover: Merge<P, T, P>,
    MetavarRemover: VisitMut<P>,
    InsProjection: Convert<P, T>,
{
    let standalone_diff = remove_metavars(multi_diff, source)?;
    Some(InsProjection.convert(standalone_diff))
}
