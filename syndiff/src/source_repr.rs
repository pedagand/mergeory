use crate::ast;
use crate::diff_tree::{Aligned, AlignedSeq, ChangeNode, DiffNode};
use crate::family_traits::Convert;
use crate::multi_diff_tree::{
    DelNode, InsNode, InsSeq, InsSeqNode, SpineNode, SpineSeq, SpineSeqNode,
};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::Token;

pub trait VerbatimMacro {
    fn verbatim_macro(mac: TokenStream) -> Self;
}

macro_rules! verbatim_macro_with_semi {
    {$($typ:ty),*} => {
        $(impl VerbatimMacro for $typ {
            fn verbatim_macro(mac: TokenStream) -> $typ {
                <$typ>::Verbatim(quote!(#mac;))
            }
        })*
    }
}
verbatim_macro_with_semi!(syn::Item, syn::TraitItem, syn::ImplItem, syn::ForeignItem);

impl VerbatimMacro for syn::Expr {
    fn verbatim_macro(mac: TokenStream) -> syn::Expr {
        syn::Expr::Verbatim(mac)
    }
}

impl VerbatimMacro for syn::Stmt {
    fn verbatim_macro(mac: TokenStream) -> syn::Stmt {
        syn::Stmt::Semi(syn::Expr::Verbatim(mac), <Token![;]>::default())
    }
}

pub struct ToSourceRepr;

// Implementations for converting diff_tree

impl<In, Out: VerbatimMacro> Convert<ChangeNode<In>, Out> for ToSourceRepr
where
    ToSourceRepr: Convert<In, Out>,
{
    fn convert(&mut self, input: ChangeNode<In>) -> Out {
        match input {
            ChangeNode::InPlace(node) => self.convert(node),
            ChangeNode::Ellided(metavar) => Out::verbatim_macro(quote!(mv![#metavar])),
        }
    }
}

impl<InSpine, InChange, Out: ToTokens + VerbatimMacro> Convert<DiffNode<InSpine, InChange>, Out>
    for ToSourceRepr
where
    ToSourceRepr: Convert<InSpine, Out>,
    ToSourceRepr: Convert<InChange, Out>,
{
    fn convert(&mut self, input: DiffNode<InSpine, InChange>) -> Out {
        match input {
            DiffNode::Spine(node) => self.convert(node),
            DiffNode::Changed(del, ins) => {
                let del: Out = self.convert(del);
                let ins: Out = self.convert(ins);
                Out::verbatim_macro(quote!(changed![{#del}, {#ins}]))
            }
            DiffNode::Unchanged(None) => Out::verbatim_macro(quote!(unchanged![])),
            DiffNode::Unchanged(Some(metavar)) => Out::verbatim_macro(quote!(unchanged![#metavar])),
        }
    }
}

impl<InSpine, InChange, Out: ToTokens + VerbatimMacro>
    Convert<AlignedSeq<InSpine, InChange>, Vec<Out>> for ToSourceRepr
where
    ToSourceRepr: Convert<DiffNode<InSpine, InChange>, Out>,
    ToSourceRepr: Convert<InChange, Out>,
{
    fn convert(&mut self, input: AlignedSeq<InSpine, InChange>) -> Vec<Out> {
        input
            .0
            .into_iter()
            .map(|elt| match elt {
                Aligned::Zipped(spine) => self.convert(spine),
                Aligned::Deleted(del) => {
                    let del: Out = self.convert(del);
                    Out::verbatim_macro(quote!(deleted![#del]))
                }
                Aligned::Inserted(ins) => {
                    let ins: Out = self.convert(ins);
                    Out::verbatim_macro(quote!(inserted![#ins]))
                }
            })
            .collect()
    }
}

// Implementations to convert multi_diff_tree

impl<D, I, O> Convert<DelNode<D, I>, O> for ToSourceRepr
where
    ToSourceRepr: Convert<D, O>,
    ToSourceRepr: Convert<InsNode<I>, O>,
    O: ToTokens + VerbatimMacro,
{
    fn convert(&mut self, input: DelNode<D, I>) -> O {
        match input {
            DelNode::InPlace(del) => self.convert(del),
            DelNode::Ellided(mv) => O::verbatim_macro(quote!(mv![#mv])),
            DelNode::MetavariableConflict(mv, del, ins) => {
                let del = Convert::<DelNode<D, I>, O>::convert(self, *del);
                let ins = self.convert(ins);
                O::verbatim_macro(quote!(mv_conflict![#mv, {#del}, {#ins}]))
            }
        }
    }
}

impl<I, O> Convert<InsNode<I>, O> for ToSourceRepr
where
    ToSourceRepr: Convert<I, O>,
    O: ToTokens + VerbatimMacro,
{
    fn convert(&mut self, input: InsNode<I>) -> O {
        match input {
            InsNode::InPlace(ins) => self.convert(ins.node),
            InsNode::Ellided(mv) => {
                let mv = mv.node;
                O::verbatim_macro(quote!(mv![#mv]))
            }
            InsNode::Conflict(conflict) => {
                let ins_iter = conflict
                    .into_iter()
                    .map(|ins| Convert::<InsNode<I>, O>::convert(self, ins));
                O::verbatim_macro(quote!(conflict![#({#ins_iter}),*]))
            }
        }
    }
}

impl<I, O> Convert<InsSeq<I>, Vec<O>> for ToSourceRepr
where
    ToSourceRepr: Convert<InsNode<I>, O>,
    O: ToTokens + VerbatimMacro,
{
    fn convert(&mut self, input: InsSeq<I>) -> Vec<O> {
        input
            .0
            .into_iter()
            .map(|node| match node {
                InsSeqNode::Node(node) => self.convert(node),
                InsSeqNode::DeleteConflict(ins) => {
                    let ins = self.convert(ins);
                    O::verbatim_macro(quote!(delete_conflict![#ins]))
                }
                InsSeqNode::InsertOrderConflict(ins_vec) => {
                    let ins_seq_iter = ins_vec.into_iter().map(|ins_seq| {
                        let ins_iter = ins_seq.node.into_iter().map(|ins| self.convert(ins));
                        quote!(#(#ins_iter)*)
                    });
                    O::verbatim_macro(quote!(insert_order_conflict![#({#ins_seq_iter}),*]))
                }
            })
            .collect()
    }
}

impl<S, D, I, O> Convert<SpineNode<S, D, I>, O> for ToSourceRepr
where
    ToSourceRepr: Convert<S, O>,
    ToSourceRepr: Convert<DelNode<D, I>, O>,
    ToSourceRepr: Convert<InsNode<I>, O>,
    O: ToTokens + VerbatimMacro,
{
    fn convert(&mut self, input: SpineNode<S, D, I>) -> O {
        match input {
            SpineNode::Spine(spine) => self.convert(spine),
            SpineNode::Unchanged => O::verbatim_macro(quote!(unchanged![])),
            SpineNode::Changed(DelNode::Ellided(del_mv), InsNode::Ellided(ins_mv))
                if del_mv == ins_mv.node =>
            {
                O::verbatim_macro(quote!(unchanged![#del_mv]))
            }
            SpineNode::Changed(del, ins) => {
                let del = self.convert(del);
                let ins = self.convert(ins);
                O::verbatim_macro(quote!(changed![{#del}, {#ins}]))
            }
        }
    }
}

impl<S, D, I, O> Convert<SpineSeq<S, D, I>, Vec<O>> for ToSourceRepr
where
    ToSourceRepr: Convert<SpineNode<S, D, I>, O>,
    ToSourceRepr: Convert<DelNode<D, I>, O>,
    ToSourceRepr: Convert<InsNode<I>, O>,
    O: ToTokens + VerbatimMacro,
{
    fn convert(&mut self, input: SpineSeq<S, D, I>) -> Vec<O> {
        input
            .0
            .into_iter()
            .map(|node| match node {
                SpineSeqNode::Zipped(spine) => self.convert(spine),
                SpineSeqNode::Deleted(del) => {
                    let del = self.convert(del.node);
                    O::verbatim_macro(quote!(deleted![#del]))
                }
                SpineSeqNode::DeleteConflict(del, ins) => {
                    let del = self.convert(del.node);
                    let ins = self.convert(ins);
                    O::verbatim_macro(quote!(delete_conflict![{#del}, {#ins}]))
                }
                SpineSeqNode::Inserted(ins_seq) => {
                    let ins_iter = ins_seq.node.into_iter().map(|ins| self.convert(ins));
                    O::verbatim_macro(quote!(inserted![#(#ins_iter)*]))
                }
                SpineSeqNode::InsertOrderConflict(ins_vec) => {
                    let ins_seq_iter = ins_vec.into_iter().map(|ins_seq| {
                        let ins_iter = ins_seq.node.into_iter().map(|ins| self.convert(ins));
                        quote!(#(#ins_iter)*)
                    });
                    O::verbatim_macro(quote!(insert_order_conflict![#({#ins_seq_iter}),*]))
                }
            })
            .collect()
    }
}

// Miscellaneous implementations to expand ignored stuff

impl Convert<String, TokenStream> for ToSourceRepr {
    fn convert(&mut self, input: String) -> TokenStream {
        TokenStream::from_str(&input).unwrap()
    }
}

impl Convert<String, proc_macro2::Literal> for ToSourceRepr {
    fn convert(&mut self, input: String) -> proc_macro2::Literal {
        match TokenStream::from_str(&input).unwrap().into_iter().next() {
            Some(proc_macro2::TokenTree::Literal(lit)) => lit,
            _ => panic!("Non literal token found at literal position"),
        }
    }
}

impl Convert<(), Span> for ToSourceRepr {
    fn convert(&mut self, _: ()) -> Span {
        Span::call_site()
    }
}

// We need to add semicolons to non-terminal Stmt's changed into macros
macro_rules! convert_block {
    { $($in_typ:ty,)* } => {
        $(impl Convert<$in_typ, syn::Block> for ToSourceRepr {
            fn convert(&mut self, input: $in_typ) -> syn::Block {
                let mut stmts: Vec<syn::Stmt> = self.convert(input.stmts);
                let nb_nontrail_stmts = stmts.len().saturating_sub(1);
                for stmt in &mut stmts[..nb_nontrail_stmts] {
                    if let syn::Stmt::Expr(syn::Expr::Verbatim(macro_call)) = stmt {
                        *stmt = syn::Stmt::Semi(
                            syn::Expr::Verbatim(std::mem::take(macro_call)),
                            <Token![;]>::default(),
                        )
                    }
                }
                syn::Block {
                    brace_token: input.brace_token,
                    stmts,
                }
            }
        })*
    }
}
convert_block!(
    ast::diff::change::Block,
    ast::diff::Block,
    ast::multi_diff::del::Block,
    ast::multi_diff::ins::Block,
    ast::multi_diff::Block,
);

// Workaround to be able to recreate the raw field of private type Reserved
macro_rules! convert_expr_reference {
    { $($in_typ:ty,)* } => {
        $(impl Convert<$in_typ, syn::ExprReference> for ToSourceRepr {
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
    ast::diff::change::ExprReference,
    ast::diff::ExprReference,
    ast::multi_diff::del::ExprReference,
    ast::multi_diff::ins::ExprReference,
    ast::multi_diff::ExprReference,
);

pub fn source_repr<In, Out>(input: In) -> Out
where
    ToSourceRepr: Convert<In, Out>,
{
    ToSourceRepr.convert(input)
}
