use mrsop_codegen::syn_codegen;

syn_codegen! {
    pub(crate) mod hash {
        use crate::convert::Convert;
        use crate::hash_tree::{HashTagged, TreeHasher};

        #[derive(Hash, PartialEq, Eq)]
        extend_family! {
            Expr as HashTagged<Expr>,
            Vec<Stmt> as Vec<HashTagged<Stmt>>,
            Vec<Item> as Vec<HashTagged<Item>>,
            Vec<TraitItem> as Vec<HashTagged<TraitItem>>,
            Vec<ImplItem> as Vec<HashTagged<ImplItem>>,
            Vec<ForeignItem> as Vec<HashTagged<ForeignItem>>,

            // TokenStream and Literal are non parsed part of the input
            // We use their string representation for hashing
            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,

            // We need to remove these subtrees to be able to compare programs
            // Span represent input file positions
            proc_macro2::Span as (),
            // Reserved is a private type inside syn equivalent to ()
            Reserved as (),
        }

        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span)]
        #[extra_call(Reserved)]
        family_impl!(Convert<syn, self> for TreeHasher);
    }

    pub(crate) mod ellided {
        use crate::visit::Visit;
        use crate::convert::Convert;
        use crate::ellided_tree::{Ellider, MaybeEllided, WantedEllisionFinder};

        extend_family! {
            Expr as MaybeEllided<Expr>,
            Vec<Stmt> as Vec<MaybeEllided<Stmt>>,
            Vec<Item> as Vec<MaybeEllided<Item>>,
            Vec<TraitItem> as Vec<MaybeEllided<TraitItem>>,
            Vec<ImplItem> as Vec<MaybeEllided<ImplItem>>,
            Vec<ForeignItem> as Vec<MaybeEllided<ForeignItem>>,

            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Visit<super::hash> for WantedEllisionFinder<'_>);
        family_impl!(Convert<super::hash, self> for Ellider<'_>);
    }

    pub(crate) mod weighted {
        use crate::convert::Convert;
        use crate::weighted_tree::{AlignableSeq, ComputeWeight, ForgetWeight, Weighted};

        extend_family! {
            Expr as Weighted<Expr>,
            Vec<Stmt> as AlignableSeq<Stmt>,
            Vec<Item> as AlignableSeq<Item>,
            Vec<TraitItem> as AlignableSeq<TraitItem>,
            Vec<ImplItem> as AlignableSeq<ImplItem>,
            Vec<ForeignItem> as AlignableSeq<ForeignItem>,

            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Convert<super::ellided, self> for ComputeWeight);
        family_impl!(Convert<self, super::ellided> for ForgetWeight);
    }

    pub(crate) mod spine {
        use crate::merge::Merge;
        use crate::spine_tree::{DiffNode, SpineZipper, AlignedSeq};

        extend_family! {
            Expr as DiffNode<Expr, super::ellided::Expr>,
            Vec<Stmt> as AlignedSeq<Stmt, super::ellided::Stmt>,
            Vec<Item> as AlignedSeq<Item, super::ellided::Item>,
            Vec<TraitItem> as AlignedSeq<TraitItem, super::ellided::TraitItem>,
            Vec<ImplItem> as AlignedSeq<ImplItem, super::ellided::ImplItem>,
            Vec<ForeignItem> as AlignedSeq<ForeignItem, super::ellided::ForeignItem>,

            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Merge<super::weighted, super::weighted, self> for SpineZipper);
    }

    pub mod change {
        use crate::visit::Visit;
        use crate::convert::Convert;
        use crate::diff_tree::{MaybeEllided, MetavariableNamer};
        use crate::source_repr::ToSourceRepr;

        extend_family! {
            Expr as MaybeEllided<Expr>,
            Vec<Stmt> as Vec<MaybeEllided<Stmt>>,
            Vec<Item> as Vec<MaybeEllided<Item>>,
            Vec<TraitItem> as Vec<MaybeEllided<TraitItem>>,
            Vec<ImplItem> as Vec<MaybeEllided<ImplItem>>,
            Vec<ForeignItem> as Vec<MaybeEllided<ForeignItem>>,

            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Visit<super::ellided> for MetavariableNamer);
        family_impl!(Convert<super::ellided, self> for MetavariableNamer);

        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span)]
        #[extra_call(Reserved)]
        #[omit(ExprReference)]
        family_impl!(Convert<self, syn> for ToSourceRepr);
    }

    pub mod diff {
        use crate::visit::Visit;
        use crate::convert::Convert;
        use crate::diff_tree::{DiffNode, AlignedSeq, MetavariableNamer};
        use crate::source_repr::ToSourceRepr;

        extend_family! {
            Expr as DiffNode<Expr, super::change::Expr>,
            Vec<Stmt> as AlignedSeq<Stmt, super::change::Stmt>,
            Vec<Item> as AlignedSeq<Item, super::change::Item>,
            Vec<TraitItem> as AlignedSeq<TraitItem, super::change::TraitItem>,
            Vec<ImplItem> as AlignedSeq<ImplItem, super::change::ImplItem>,
            Vec<ForeignItem> as AlignedSeq<ForeignItem, super::change::ForeignItem>,

            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Visit<super::spine> for MetavariableNamer);
        family_impl!(Convert<super::spine, self> for MetavariableNamer);

        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span)]
        #[extra_call(Reserved)]
        #[omit(ExprReference)]
        family_impl!(Convert<self, syn> for ToSourceRepr);
    }
}
