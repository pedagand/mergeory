use mrsop_codegen::syn_codegen;

syn_codegen! {
    pub mod hash {
        use crate::convert::Convert;
        use crate::hash_tree::{HashTagged, TreeHasher};

        #[derive(Hash, PartialEq, Eq, Debug)]
        extend_family! {
            Expr as HashTagged<Expr>,
            Item as HashTagged<Item>,
            Stmt as HashTagged<Stmt>,

            // We need to remove these subtrees to be able to compare programs
            // TokenStream and Literal are non parsed part of the input
            proc_macro2::TokenStream as (),
            proc_macro2::Literal as (),
            // Span represent input file positions
            proc_macro2::Span as (),
            // Reserved is a private type inside syn equivalent to ()
            Reserved as (),
        }

        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span)]
        #[extra_call(Reserved)]
        family_impl!(Convert<syn, self> for TreeHasher);
    }

    pub mod ellided {
        use crate::visit::Visit;
        use crate::convert::Convert;
        use crate::ellided_tree::{Ellider, MaybeEllided, WantedEllisionFinder};
        use crate::source_repr::ToSourceRepr;

        #[derive(Debug)]
        extend_family! {
            Expr as MaybeEllided<Expr>,
            Item as MaybeEllided<Item>,
            Stmt as MaybeEllided<Stmt>,

            proc_macro2::TokenStream as (),
            proc_macro2::Literal as (),
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Visit<super::hash> for WantedEllisionFinder<'_>);
        family_impl!(Convert<super::hash, self> for Ellider<'_>);

        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span)]
        #[extra_call(Reserved)]
        #[omit(ExprReference)]
        family_impl!(Convert<self, syn> for ToSourceRepr);
    }

    pub mod weighted {
        use crate::convert::Convert;
        use crate::weighted_tree::{AlignableSeq, ComputeWeight, ForgetWeight, Weighted};

        extend_family! {
            Expr as Weighted<Expr>,
            Item as Weighted<Item>,
            Stmt as Weighted<Stmt>,
            Vec<Item> as AlignableSeq<Item>,
            Vec<Stmt> as AlignableSeq<Stmt>,

            proc_macro2::TokenStream as (),
            proc_macro2::Literal as (),
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Convert<super::ellided, self> for ComputeWeight);
        family_impl!(Convert<self, super::ellided> for ForgetWeight);
    }

    pub mod patch {
        use crate::convert::Convert;
        use crate::merge::Merge;
        use crate::patch_tree::{DiffNode, SpineZipper, AlignedSeq};
        use crate::source_repr::ToSourceRepr;

        #[derive(Debug)]
        extend_family! {
            Expr as DiffNode<Expr, super::ellided::Expr>,
            Item as DiffNode<Item, super::ellided::Item>,
            Stmt as DiffNode<Stmt, super::ellided::Stmt>,
            Vec<Item> as AlignedSeq<Item, super::ellided::Item>,
            Vec<Stmt> as AlignedSeq<Stmt, super::ellided::Stmt>,

            proc_macro2::TokenStream as (),
            proc_macro2::Literal as (),
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Merge<super::weighted, super::weighted, self> for SpineZipper);

        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span)]
        #[extra_call(Reserved)]
        #[omit(ExprReference)]
        family_impl!(Convert<self, syn> for ToSourceRepr);
    }
}
