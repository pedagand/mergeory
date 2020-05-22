use crate::ast;
use crate::diff_tree::{Aligned, AlignedSeq, ChangeNode, DiffNode};
use crate::family_traits::Convert;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::{LitInt, Token};

pub trait VerbatimMacro {
    fn verbatim_macro(mac: TokenStream) -> Self;
}

macro_rules! verbatim_macro_without_semi {
    {$($typ:ty),*} => {
        $(impl VerbatimMacro for $typ {
            fn verbatim_macro(mac: TokenStream) -> $typ {
                <$typ>::Verbatim(mac)
            }
        })*
    }
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

verbatim_macro_without_semi!(syn::Expr);
verbatim_macro_with_semi!(syn::Item, syn::TraitItem, syn::ImplItem, syn::ForeignItem);

impl VerbatimMacro for syn::Stmt {
    fn verbatim_macro(mac: TokenStream) -> syn::Stmt {
        syn::Stmt::Semi(syn::Expr::Verbatim(mac), <Token![;]>::default())
    }
}

pub struct ToSourceRepr;

impl<In, Out: VerbatimMacro> Convert<ChangeNode<In>, Out> for ToSourceRepr
where
    ToSourceRepr: Convert<In, Out>,
{
    fn convert(&mut self, input: ChangeNode<In>) -> Out {
        match input {
            ChangeNode::InPlace(node) => self.convert(node),
            ChangeNode::Ellided(metavar) => {
                let metavar_lit = LitInt::new(&format!("{}", metavar), Span::call_site());
                Out::verbatim_macro(quote!(metavar![#metavar_lit]))
            }
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
            DiffNode::Unchanged(Some(metavar)) => {
                let metavar_lit = LitInt::new(&format!("{}", metavar), Span::call_site());
                Out::verbatim_macro(quote!(unchanged![#metavar_lit]))
            }
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
    { $($in_typ:ty),* } => {
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
convert_block!(ast::diff::change::Block, ast::diff::Block);

// Workaround to be able to recreate the raw field of private type Reserved
macro_rules! convert_expr_reference {
    { $($in_typ:ty),* } => {
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
convert_expr_reference!(ast::diff::change::ExprReference, ast::diff::ExprReference);

pub fn source_repr<In, Out>(input: In) -> Out
where
    ToSourceRepr: Convert<In, Out>,
{
    ToSourceRepr.convert(input)
}
