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
    }
}
