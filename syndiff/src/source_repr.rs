use crate::ast;
use crate::diff_tree::{Aligned, AlignedSeq, ChangeNode, DiffNode, Metavariable};
use crate::family_traits::Convert;
use crate::multi_diff_tree::{
    ColorSet, Colored, DelNode, InsNode, InsSeq, InsSeqNode, SpineNode, SpineSeq, SpineSeqNode,
};
use crate::token_trees::{iter_token_trees, TokenTree};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::punctuated::Punctuated;
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

impl VerbatimMacro for syn::Attribute {
    fn verbatim_macro(mac: TokenStream) -> syn::Attribute {
        use syn::parse_quote;
        // FIXME: This is flawed!
        // We should use the same style (outer/inner) as the original attribute
        parse_quote!(#[diff = #mac])
    }
}

impl VerbatimMacro for syn::Arm {
    fn verbatim_macro(mac: TokenStream) -> syn::Arm {
        use syn::parse_quote;
        parse_quote!(#mac => match_arm![],)
    }
}

impl VerbatimMacro for TokenTree {
    fn verbatim_macro(mac: TokenStream) -> TokenTree {
        TokenTree::Group(
            proc_macro2::Delimiter::None,
            iter_token_trees(mac).collect(),
        )
    }
}

impl VerbatimMacro for proc_macro2::TokenTree {
    fn verbatim_macro(mac: TokenStream) -> proc_macro2::TokenTree {
        proc_macro2::Group::new(proc_macro2::Delimiter::None, mac).into()
    }
}

// FIXME: This implementation prints differences outside of the tree since
// we cannot always generate a macro at any point of the code.
impl<T> VerbatimMacro for Option<T> {
    fn verbatim_macro(mac: TokenStream) -> Option<T> {
        println!("{}", mac);
        None
    }
}

pub struct ToSourceRepr(Option<ColorSet>);

impl<O> Convert<Metavariable, O> for ToSourceRepr
where
    O: VerbatimMacro,
{
    fn convert(&mut self, mv: Metavariable) -> O {
        O::verbatim_macro(quote!(mv![#mv]))
    }
}

macro_rules! impl_convert_for_seq {
    ($seq:ident<$($T:ident),*> : $it:ty) => {
        impl<$($T,)* O> Convert<$seq<$($T),*>, Vec<O>> for ToSourceRepr
        where
            ToSourceRepr: Convert<Vec<$it>, Vec<O>>,
        {
            fn convert(&mut self, input: $seq<$($T),*>) -> Vec<O> {
                self.convert(input.0)
            }
        }

        impl<$($T),*> Convert<$seq<$($T),*>, TokenStream> for ToSourceRepr
        where
            ToSourceRepr: Convert<Vec<$it>, TokenStream>,
        {
            fn convert(&mut self, input: $seq<$($T),*>) -> TokenStream {
                self.convert(input.0)
            }
        }
    };
}

// Implementations for converting diff_tree

impl<I, O> Convert<ChangeNode<I>, O> for ToSourceRepr
where
    ToSourceRepr: Convert<I, O>,
    O: VerbatimMacro,
{
    fn convert(&mut self, input: ChangeNode<I>) -> O {
        match input {
            ChangeNode::InPlace(node) => self.convert(node),
            ChangeNode::Elided(mv) => Convert::<Metavariable, O>::convert(self, mv),
        }
    }
}

// FIXME: This implementation prints differences outside of the tree since
// we cannot always generate a macro at any point of the code.
impl<I, O, P> Convert<Vec<ChangeNode<I>>, Punctuated<O, P>> for ToSourceRepr
where
    ToSourceRepr: Convert<I, O>,
    O: ToTokens,
    P: Default,
{
    fn convert(&mut self, input: Vec<ChangeNode<I>>) -> Punctuated<O, P> {
        input
            .into_iter()
            .filter_map(|elt| match elt {
                ChangeNode::InPlace(node) => Some(self.convert(node)),
                ChangeNode::Elided(mv) => {
                    println!("mv![{}]", mv);
                    None
                }
            })
            .collect()
    }
}

impl<S, C, O> Convert<DiffNode<S, C>, O> for ToSourceRepr
where
    ToSourceRepr: Convert<S, O>,
    ToSourceRepr: Convert<C, O>,
    O: ToTokens + VerbatimMacro,
{
    fn convert(&mut self, input: DiffNode<S, C>) -> O {
        match input {
            DiffNode::Spine(node) => self.convert(node),
            DiffNode::Changed(del, ins) => {
                let del: O = self.convert(del);
                let ins: O = self.convert(ins);
                O::verbatim_macro(quote!(changed![{#del}, {#ins}]))
            }
            DiffNode::Unchanged(None) => O::verbatim_macro(quote!(unchanged![])),
            DiffNode::Unchanged(Some(mv)) => O::verbatim_macro(quote!(unchanged![#mv])),
        }
    }
}

impl<S, C, O> Convert<Aligned<S, C>, O> for ToSourceRepr
where
    ToSourceRepr: Convert<DiffNode<S, C>, O>,
    ToSourceRepr: Convert<C, O>,
    O: ToTokens + VerbatimMacro,
{
    fn convert(&mut self, input: Aligned<S, C>) -> O {
        match input {
            Aligned::Zipped(spine) => self.convert(spine),
            Aligned::Deleted(del) => {
                let del: O = self.convert(del);
                O::verbatim_macro(quote!(deleted![#del]))
            }
            Aligned::Inserted(ins) => {
                let ins: O = self.convert(ins);
                O::verbatim_macro(quote!(inserted![#ins]))
            }
        }
    }
}

impl_convert_for_seq!(AlignedSeq<S, C>: Aligned<S, C>);

fn change_node_as_string<I, O>(converter: &mut ToSourceRepr, input: ChangeNode<I>) -> String
where
    ToSourceRepr: Convert<I, O>,
    O: ToTokens,
{
    match input {
        ChangeNode::InPlace(c) => format!("{}", converter.convert(c).into_token_stream()),
        ChangeNode::Elided(mv) => format!("mv![{}]", mv),
    }
}

// FIXME: This implementation prints differences outside of the tree since
// we cannot always generate a macro at any point of the code.
impl<S, C, O, P> Convert<AlignedSeq<S, C>, Punctuated<O, P>> for ToSourceRepr
where
    ToSourceRepr: Convert<S, O>,
    ToSourceRepr: Convert<C, O>,
    O: ToTokens,
    P: Default,
{
    fn convert(&mut self, input: AlignedSeq<S, C>) -> Punctuated<O, P> {
        input
            .0
            .into_iter()
            .filter_map::<O, _>(|elt| match elt {
                Aligned::Zipped(DiffNode::Spine(spine)) => Some(self.convert(spine)),
                Aligned::Zipped(DiffNode::Changed(del, ins)) => {
                    println!(
                        "changed![{{{}}}, {{{}}}]",
                        change_node_as_string(self, del),
                        change_node_as_string(self, ins)
                    );
                    None
                }
                Aligned::Zipped(DiffNode::Unchanged(_)) => None,
                Aligned::Deleted(del) => {
                    println!("deleted![{}]", change_node_as_string(self, del));
                    None
                }
                Aligned::Inserted(ins) => {
                    println!("inserted![{}]", change_node_as_string(self, ins));
                    None
                }
            })
            .collect()
    }
}

// Implementations to convert multi_diff_tree

impl<T, O> Convert<Colored<T>, O> for ToSourceRepr
where
    ToSourceRepr: Convert<T, O>,
    O: ToTokens + VerbatimMacro,
{
    fn convert(&mut self, input: Colored<T>) -> O {
        match self.0 {
            None => self.convert(input.node),
            Some(col) if input.colors == col => self.convert(input.node),
            Some(_) => {
                let color_set = input.colors;
                let node = ToSourceRepr(Some(color_set)).convert(input.node);
                O::verbatim_macro(quote!(colored![#color_set, {#node}]))
            }
        }
    }
}

impl<D, I, O> Convert<DelNode<D, I>, O> for ToSourceRepr
where
    ToSourceRepr: Convert<Colored<D>, O>,
    ToSourceRepr: Convert<InsNode<I>, O>,
    O: ToTokens + VerbatimMacro,
{
    fn convert(&mut self, input: DelNode<D, I>) -> O {
        match input {
            DelNode::InPlace(del) => self.convert(del),
            DelNode::Elided(mv) => self.convert(mv),
            DelNode::MetavariableConflict(mv, del, repl) => {
                let del = Convert::<DelNode<D, I>, O>::convert(self, *del);
                match repl {
                    None => O::verbatim_macro(quote!(mv_conflict![#mv, {#del}])),
                    Some(ins) => {
                        let ins = self.convert(ins);
                        O::verbatim_macro(quote!(mv_conflict![#mv, {#del}, {#ins}]))
                    }
                }
            }
        }
    }
}

impl<I, O> Convert<InsNode<I>, O> for ToSourceRepr
where
    ToSourceRepr: Convert<Colored<I>, O>,
    O: ToTokens + VerbatimMacro,
{
    fn convert(&mut self, input: InsNode<I>) -> O {
        match input {
            InsNode::InPlace(ins) => self.convert(ins),
            InsNode::Elided(mv) => Convert::<Metavariable, O>::convert(self, mv),
            InsNode::Conflict(conflict) => {
                let ins_iter = conflict
                    .into_iter()
                    .map(|ins| Convert::<InsNode<I>, O>::convert(self, ins));
                O::verbatim_macro(quote!(conflict![#({#ins_iter}),*]))
            }
        }
    }
}

impl<I, O> Convert<InsSeqNode<I>, O> for ToSourceRepr
where
    ToSourceRepr: Convert<InsNode<I>, O>,
    O: ToTokens + VerbatimMacro,
{
    fn convert(&mut self, input: InsSeqNode<I>) -> O {
        match input {
            InsSeqNode::Node(node) => self.convert(node),
            InsSeqNode::DeleteConflict(ins) => {
                let ins = self.convert(ins);
                O::verbatim_macro(quote!(delete_conflict![#ins]))
            }
            InsSeqNode::InsertOrderConflict(ins_vec) => {
                let ins_seq_iter = ins_vec.into_iter().map(|ins_seq| {
                    if self.0.is_none() {
                        let ins_iter = ins_seq.node.into_iter().map(|ins| self.convert(ins));
                        quote!({#(#ins_iter)*})
                    } else {
                        let ins_colors = ins_seq.colors;
                        let ins_iter = ins_seq
                            .node
                            .into_iter()
                            .map(|ins| ToSourceRepr(Some(ins_colors)).convert(ins));
                        quote!((#ins_colors, {#(#ins_iter)*}))
                    }
                });
                O::verbatim_macro(quote!(insert_order_conflict![#(#ins_seq_iter),*]))
            }
        }
    }
}

impl_convert_for_seq!(InsSeq<I>: InsSeqNode<I>);

fn ins_node_as_string<I, O>(converter: &mut ToSourceRepr, input: InsNode<I>) -> String
where
    ToSourceRepr: Convert<I, O>,
    O: ToTokens,
{
    match input {
        InsNode::InPlace(i) => format!("{}", converter.convert(i.node).into_token_stream()),
        InsNode::Elided(mv) => format!("mv![{}]", mv),
        InsNode::Conflict(confl) => {
            let confl_tokens = confl
                .into_iter()
                .map(|n| format!("{}", ins_node_as_string(converter, n).into_token_stream()))
                .collect::<Vec<_>>()
                .join(", ");
            format!("conflict![{}]", confl_tokens)
        }
    }
}

// FIXME: Dirty printing impl
impl<I, O, P> Convert<InsSeq<I>, Punctuated<O, P>> for ToSourceRepr
where
    ToSourceRepr: Convert<I, O>,
    O: ToTokens,
    P: Default + ToTokens,
{
    fn convert(&mut self, input: InsSeq<I>) -> Punctuated<O, P> {
        input
            .0
            .into_iter()
            .filter_map(|node| match node {
                InsSeqNode::Node(InsNode::InPlace(i)) => Some(self.convert(i.node)),
                InsSeqNode::Node(node) => {
                    println!("{}", ins_node_as_string(self, node));
                    None
                }
                InsSeqNode::DeleteConflict(ins) => {
                    println!("delete_conflict![{}]", ins_node_as_string(self, ins));
                    None
                }
                InsSeqNode::InsertOrderConflict(ins_vec) => {
                    let punct = format!("{}", P::default().into_token_stream());
                    let ins_seq_str = ins_vec
                        .into_iter()
                        .map(|ins_seq| {
                            format!(
                                "{{{}}}",
                                ins_seq
                                    .node
                                    .into_iter()
                                    .map(|ins| ins_node_as_string(self, ins))
                                    .collect::<Vec<_>>()
                                    .join(&punct)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    println!("insert_order_conflict![{}]", ins_seq_str);
                    None
                }
            })
            .collect()
    }
}

fn del_node_as_string<D, I, O>(converter: &mut ToSourceRepr, input: DelNode<D, I>) -> String
where
    ToSourceRepr: Convert<D, O>,
    ToSourceRepr: Convert<I, O>,
    O: ToTokens,
{
    match input {
        DelNode::InPlace(i) => format!("{}", converter.convert(i.node).into_token_stream()),
        DelNode::Elided(mv) => format!("mv![{}]", mv.node),
        DelNode::MetavariableConflict(mv, del, subst) => {
            let del = del_node_as_string(converter, *del);
            match subst {
                None => format!("mv_conflict![{}, {{{}}}])", mv, del),
                Some(ins) => {
                    let ins = ins_node_as_string(converter, ins);
                    format!("mv_conflict![{}, {{{}}}, {{{}}}])", mv, del, ins)
                }
            }
        }
    }
}

// FIXME: Dirty printing impl
impl<D, I, O, P> Convert<Vec<DelNode<D, I>>, Punctuated<O, P>> for ToSourceRepr
where
    ToSourceRepr: Convert<D, O>,
    ToSourceRepr: Convert<I, O>,
    O: ToTokens,
    P: Default,
{
    fn convert(&mut self, input: Vec<DelNode<D, I>>) -> Punctuated<O, P> {
        input
            .into_iter()
            .filter_map(|node| match node {
                DelNode::InPlace(del) => Some(self.convert(del.node)),
                _ => {
                    println!("{}", del_node_as_string(self, node));
                    None
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
            SpineNode::Changed(DelNode::Elided(del_mv), InsNode::Elided(ins_mv))
                if del_mv.node == ins_mv =>
            {
                O::verbatim_macro(quote!(unchanged![#ins_mv]))
            }
            SpineNode::Changed(del, ins) => {
                let del = self.convert(del);
                let ins = self.convert(ins);
                O::verbatim_macro(quote!(changed![{#del}, {#ins}]))
            }
        }
    }
}

impl<S, D, I, O> Convert<SpineSeqNode<S, D, I>, O> for ToSourceRepr
where
    ToSourceRepr: Convert<SpineNode<S, D, I>, O>,
    ToSourceRepr: Convert<DelNode<D, I>, O>,
    ToSourceRepr: Convert<InsNode<I>, O>,
    O: ToTokens + VerbatimMacro,
{
    fn convert(&mut self, input: SpineSeqNode<S, D, I>) -> O {
        match input {
            SpineSeqNode::Zipped(spine) => self.convert(spine),
            SpineSeqNode::Deleted(del) => {
                let del = self.convert(del);
                O::verbatim_macro(quote!(deleted![#del]))
            }
            SpineSeqNode::DeleteConflict(del, ins) => {
                let del = self.convert(del);
                let ins = self.convert(ins);
                O::verbatim_macro(quote!(delete_conflict![{#del}, {#ins}]))
            }
            SpineSeqNode::Inserted(ins_seq) => {
                if self.0.is_none() {
                    let ins_iter = ins_seq.node.into_iter().map(|ins| self.convert(ins));
                    O::verbatim_macro(quote!(inserted![#(#ins_iter)*]))
                } else {
                    let ins_colors = ins_seq.colors;
                    let ins_iter = ins_seq
                        .node
                        .into_iter()
                        .map(|ins| ToSourceRepr(Some(ins_colors)).convert(ins));
                    O::verbatim_macro(quote!(inserted![#ins_colors, {#(#ins_iter)*}]))
                }
            }
            SpineSeqNode::InsertOrderConflict(ins_vec) => {
                let ins_seq_iter = ins_vec.into_iter().map(|ins_seq| {
                    if self.0.is_none() {
                        let ins_iter = ins_seq.node.into_iter().map(|ins| self.convert(ins));
                        quote!({#(#ins_iter)*})
                    } else {
                        let ins_colors = ins_seq.colors;
                        let ins_iter = ins_seq
                            .node
                            .into_iter()
                            .map(|ins| ToSourceRepr(Some(ins_colors)).convert(ins));
                        quote!((#ins_colors, {#(#ins_iter)*}))
                    }
                });
                O::verbatim_macro(quote!(insert_order_conflict![#(#ins_seq_iter),*]))
            }
        }
    }
}

impl_convert_for_seq!(SpineSeq<S, D, I>: SpineSeqNode<S, D, I>);

// FIXME: Dirty prints
impl<S, D, I, O, P> Convert<SpineSeq<S, D, I>, Punctuated<O, P>> for ToSourceRepr
where
    ToSourceRepr: Convert<S, O>,
    ToSourceRepr: Convert<D, O>,
    ToSourceRepr: Convert<I, O>,
    O: ToTokens,
    P: Default + ToTokens,
{
    fn convert(&mut self, input: SpineSeq<S, D, I>) -> Punctuated<O, P> {
        input
            .0
            .into_iter()
            .filter_map(|node| match node {
                SpineSeqNode::Zipped(SpineNode::Spine(spine)) => Some(self.convert(spine)),
                SpineSeqNode::Zipped(SpineNode::Unchanged) => None,
                SpineSeqNode::Zipped(SpineNode::Changed(del, ins)) => {
                    println!(
                        "changed![{{{}}}, {{{}}}]",
                        del_node_as_string(self, del),
                        ins_node_as_string(self, ins)
                    );
                    None
                }
                SpineSeqNode::Deleted(del) => {
                    println!("deleted![{}]", del_node_as_string(self, del));
                    None
                }
                SpineSeqNode::DeleteConflict(del, ins) => {
                    println!(
                        "delete_conflict![{{{}}}, {{{}}}]",
                        del_node_as_string(self, del),
                        ins_node_as_string(self, ins)
                    );
                    None
                }
                SpineSeqNode::Inserted(ins_seq) => {
                    let punct = format!("{}", P::default().into_token_stream());
                    let ins_seq_str = ins_seq
                        .node
                        .into_iter()
                        .map(|ins| ins_node_as_string(self, ins))
                        .collect::<Vec<_>>()
                        .join(&punct);
                    println!("inserted![{}]", ins_seq_str);
                    None
                }
                SpineSeqNode::InsertOrderConflict(ins_vec) => {
                    let punct = format!("{}", P::default().into_token_stream());
                    let ins_seq_str = ins_vec
                        .into_iter()
                        .map(|ins_seq| {
                            format!(
                                "{{{}}}",
                                ins_seq
                                    .node
                                    .into_iter()
                                    .map(|ins| ins_node_as_string(self, ins))
                                    .collect::<Vec<_>>()
                                    .join(&punct)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    println!("insert_order_conflict![{}]", ins_seq_str);
                    None
                }
            })
            .collect()
    }
}

// Miscellaneous implementations to expand ignored stuff

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

pub fn source_repr<I, O>(input: I) -> O
where
    ToSourceRepr: Convert<I, O>,
{
    ToSourceRepr(None).convert(input)
}

pub fn colored_source_repr<I, O>(input: I) -> O
where
    ToSourceRepr: Convert<I, O>,
{
    ToSourceRepr(Some(ColorSet::white())).convert(input)
}
