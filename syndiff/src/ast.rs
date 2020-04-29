use mrsop_codegen::syn_codegen;

syn_codegen! {
    pub mod hash {
        use crate::convert::Convert;
        use crate::hash_tree::{HashTables, HashTagged};

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
        family_impl!(Convert<syn, self> for HashTables);
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

    pub mod scoped {
        use crate::convert::Convert;
        use crate::ellided_tree::MaybeEllided;
        use crate::scoped_tree::{MetavarScope, ComputeScopes, ForgetScopes};

        extend_family! {
            Expr as MaybeEllided<MetavarScope<Expr>>,
            Item as MaybeEllided<MetavarScope<Item>>,
            Stmt as MaybeEllided<MetavarScope<Stmt>>,

            proc_macro2::TokenStream as (),
            proc_macro2::Literal as (),
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Convert<super::ellided, self> for ComputeScopes);
        family_impl!(Convert<self, super::ellided> for ForgetScopes);
    }

    pub mod patch {
        use crate::convert::Convert;
        use crate::merge::Merge;
        use crate::patch_tree::{DiffNode, SpineZipper};
        use crate::source_repr::ToSourceRepr;

        #[derive(Debug)]
        extend_family! {
            Expr as DiffNode<Expr, super::ellided::Expr>,
            Item as DiffNode<Item, super::ellided::Item>,
            Stmt as DiffNode<Stmt, super::ellided::Stmt>,

            proc_macro2::TokenStream as (),
            proc_macro2::Literal as (),
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Merge<super::scoped, super::scoped, self> for SpineZipper);

        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span)]
        #[extra_call(Reserved)]
        #[omit(ExprReference)]
        family_impl!(Convert<self, syn> for ToSourceRepr);
    }
}
