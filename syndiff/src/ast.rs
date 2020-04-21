use mrsop_codegen::syn_codegen;

syn_codegen! {
    pub mod hash_tree {
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

        #[extra_conversion(proc_macro2::TokenStream)]
        #[extra_conversion(proc_macro2::Literal)]
        #[extra_conversion(proc_macro2::Span)]
        #[extra_conversion(Reserved)]
        impl_convert!(syn -> self for HashTables);
    }
}
