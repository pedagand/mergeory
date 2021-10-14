#![allow(clippy::large_enum_variant)]

use mrsop_codegen::syn_codegen;

syn_codegen! {
    pub(crate) mod hash {
        use crate::family_traits::Convert;
        use crate::hash_tree::{HashTagged, TreeHasher};
        pub use crate::token_trees::hash::TokenTree;

        #[derive(Hash, PartialEq, Eq)]
        extend_family! {
            Expr as HashTagged<Expr>,
            for<T> Vec<T> as Vec<HashTagged<T>>,
            proc_macro2::TokenStream as Vec<HashTagged<TokenTree>>,

            // We consider literals as strings for easy hashing
            proc_macro2::Literal as String,

            // We need to remove these subtrees to be able to compare programs
            // Span represent input file positions
            proc_macro2::Span as (),
            // Reserved is a private type inside syn equivalent to ()
            Reserved as (),
        }

        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        family_impl!(Convert<syn, self> for TreeHasher);
    }

    pub(crate) mod elided {
        use crate::family_traits::{Convert, Visit};
        use crate::elided_tree::{Elider, MaybeElided, WantedElisionFinder};
        pub use crate::token_trees::elided::TokenTree;

        extend_family! {
            Expr as MaybeElided<Expr>,
            for<T> Vec<T> as Vec<MaybeElided<T>>,
            proc_macro2::TokenStream as Vec<MaybeElided<TokenTree>>,

            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Visit<super::hash> for WantedElisionFinder<'_>);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Convert<super::hash, self> for Elider<'_>);
    }

    pub(crate) mod weighted {
        use crate::family_traits::Convert;
        use crate::weighted_tree::{AlignableSeq, ComputeWeight, ForgetWeight, Weighted};
        pub use crate::token_trees::weighted::TokenTree;

        extend_family! {
            Expr as Weighted<Expr>,
            for<T> Vec<T> as AlignableSeq<T>,
            proc_macro2::TokenStream as AlignableSeq<TokenTree>,

            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_link!(self -> super::elided as WithoutWeight);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Convert<super::elided, self> for ComputeWeight);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Convert<self, super::elided> for ForgetWeight);
    }

    pub(crate) mod spine {
        use crate::family_traits::Merge;
        use crate::spine_tree::{DiffNode, SpineZipper, AlignedSeq};
        pub use crate::token_trees::spine::TokenTree;

        extend_family! {
            Expr as DiffNode<Expr, super::elided::Expr>,
            for<T> Vec<T> as AlignedSeq<T, super::elided::T>,
            proc_macro2::TokenStream as AlignedSeq<TokenTree, super::elided::TokenTree>,

            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Merge<super::weighted, super::weighted, self> for SpineZipper);
    }

    pub mod diff {
        pub mod change {
            use crate::diff_tree::ChangeNode;
            pub use crate::token_trees::diff::change::TokenTree;

            extend_family! {
                Expr as ChangeNode<Expr>,
                for<T> Vec<T> as Vec<ChangeNode<T>>,
                proc_macro2::TokenStream as Vec<ChangeNode<TokenTree>>,

                proc_macro2::Literal as String,
                proc_macro2::Span as (),
                Reserved as (),
            }
        }

        use crate::family_traits::{Convert, Visit};
        use crate::diff_tree::{AlignedSeq, DiffNode, MetavariableNamer};
        use crate::source_repr::ToSourceRepr;
        pub use crate::token_trees::diff::TokenTree;

        extend_family! {
            Expr as DiffNode<Expr, change::Expr>,
            for<T> Vec<T> as AlignedSeq<T, change::T>,
            proc_macro2::TokenStream as AlignedSeq<TokenTree, change::TokenTree>,

            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Visit<super::elided> for MetavariableNamer);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Convert<super::elided, change> for MetavariableNamer);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Visit<super::spine> for MetavariableNamer);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Convert<super::spine, self> for MetavariableNamer);

        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        #[omit(Block, ExprReference)]
        family_impl!(Convert<change, syn> for ToSourceRepr);

        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        #[omit(Block, ExprReference)]
        family_impl!(Convert<self, syn> for ToSourceRepr);
    }

    pub mod multi_diff {
        pub mod ins {
            use crate::multi_diff_tree::{InsNode, InsSeq};
            pub use crate::token_trees::multi_diff::ins::TokenTree;

            #[derive(Clone)]
            extend_family! {
                Expr as InsNode<Expr>,
                for<T> Vec<T> as InsSeq<T>,
                proc_macro2::TokenStream as InsSeq<TokenTree>,

                proc_macro2::Literal as String,
                proc_macro2::Span as (),
                Reserved as (),
            }
        }

        pub mod del {
            use crate::multi_diff_tree::DelNode;
            pub use crate::token_trees::multi_diff::del::TokenTree;

            #[derive(Clone)]
            extend_family! {
                Expr as DelNode<Expr, super::ins::Expr>,
                for<T> Vec<T> as Vec<DelNode<T, super::ins::T>>,
                proc_macro2::TokenStream as Vec<DelNode<TokenTree, super::ins::TokenTree>>,

                proc_macro2::Literal as String,
                proc_macro2::Span as (),
                Reserved as (),
            }
        }

        pub(crate) mod merge_spine {
            use crate::multi_diff_tree::align_spine::{MergeSpineNode, MergeSpineSeq};
            pub use crate::token_trees::multi_diff::merge_spine::TokenTree;

            extend_family! {
                Expr as MergeSpineNode<Expr, super::del::Expr, super::ins::Expr>,
                for<T> Vec<T> as MergeSpineSeq<T, super::del::T, super::ins::T>,
                proc_macro2::TokenStream as
                    MergeSpineSeq<TokenTree, super::del::TokenTree, super::ins::TokenTree>,

                proc_macro2::Literal as String,
                proc_macro2::Span as (),
                Reserved as (),
            }
        }

        pub(crate) mod ins_merged_spine {
            use crate::multi_diff_tree::merge_ins::{ISpineNode, ISpineSeq};
            pub use crate::token_trees::multi_diff::ins_merged_spine::TokenTree;

            extend_family! {
                Expr as ISpineNode<Expr, super::del::Expr, super::ins::Expr>,
                for<T> Vec<T> as ISpineSeq<T, super::del::T, super::ins::T>,
                proc_macro2::TokenStream as
                    ISpineSeq<TokenTree, super::del::TokenTree, super::ins::TokenTree>,

                proc_macro2::Literal as String,
                proc_macro2::Span as (),
                Reserved as (),
            }
        }

        use crate::multi_diff_tree::{SpineNode, SpineSeq};
        pub use crate::token_trees::multi_diff::TokenTree;

        extend_family! {
            Expr as SpineNode<Expr, del::Expr, ins::Expr>,
            for<T> Vec<T> as SpineSeq<T, del::T, ins::T>,
            proc_macro2::TokenStream as SpineSeq<TokenTree, del::TokenTree, ins::TokenTree>,

            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_link!(ins -> del as DelEquivType);

        use crate::family_traits::{Convert, Merge, Split, Visit, VisitMut};

        use crate::multi_diff_tree::with_color::WithColor;
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Convert<super::diff, self> for WithColor);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Convert<super::diff::change, del> for WithColor);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Convert<super::diff::change, ins> for WithColor);

        use crate::multi_diff_tree::metavar_renamer::MetavarRenamer;
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<del> for MetavarRenamer);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<ins> for MetavarRenamer);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<self> for MetavarRenamer);

        use crate::multi_diff_tree::align_spine::SpineAligner;
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Split<self, del, ins> for SpineAligner);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Merge<self, self, merge_spine> for SpineAligner);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Convert<self, merge_spine> for SpineAligner);

        use crate::multi_diff_tree::merge_ins::InsMerger;
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Merge<ins, ins, ins> for InsMerger);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Merge<del, ins, del> for InsMerger);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<del> for InsMerger);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Convert<merge_spine, ins_merged_spine> for InsMerger);

        use crate::multi_diff_tree::merge_del::{DelMerger, ColorAdder};
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Merge<del, del, del> for DelMerger);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Convert<ins_merged_spine, self> for DelMerger);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<del> for ColorAdder);

        use crate::multi_diff_tree::id_merger::IdMerger;
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Merge<ins, ins, ins> for IdMerger);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Merge<del, ins, del> for IdMerger);

        use crate::multi_diff_tree::subst::{Substituter, ColorReplacer, InferInsFromDel, SolvedConflictsRemover};
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<del> for Substituter);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<ins> for Substituter);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<self> for Substituter);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<del> for ColorReplacer);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Convert<del, ins> for InferInsFromDel);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<del> for SolvedConflictsRemover);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<self> for SolvedConflictsRemover);

        use crate::multi_diff_tree::metavar_remover::{MetavarRemover, InferFromSyn, InferFromSynColored};
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        family_impl!(Merge<self, syn, self> for MetavarRemover);
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        family_impl!(Merge<del, syn, del> for MetavarRemover);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<ins> for MetavarRemover);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<del> for MetavarRemover);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(VisitMut<self> for MetavarRemover);
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        family_impl!(Convert<syn, del> for InferFromSynColored);
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        family_impl!(Convert<syn, ins> for InferFromSyn);
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        family_impl!(Convert<syn, self> for InferFromSyn);

        use crate::multi_diff_tree::conflict_counter::ConflictCounter;
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Visit<del> for ConflictCounter);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Visit<ins> for ConflictCounter);
        #[extra_call(proc_macro2::TokenStream)]
        family_impl!(Visit<self> for ConflictCounter);

        use crate::source_repr::ToSourceRepr;
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        #[omit(Block, ExprReference)]
        family_impl!(Convert<del, syn> for ToSourceRepr);
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        #[omit(Block, ExprReference)]
        family_impl!(Convert<ins, syn> for ToSourceRepr);
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        #[omit(Block, ExprReference)]
        family_impl!(Convert<self, syn> for ToSourceRepr);

        use crate::multi_diff_tree::patch::InsProjection;
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        #[omit(ExprReference)]
        family_impl!(Convert<ins, syn> for InsProjection);
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        #[omit(ExprReference)]
        family_impl!(Convert<self, syn> for InsProjection);
        family_link!(syn -> syn as SynType);
    }
}
