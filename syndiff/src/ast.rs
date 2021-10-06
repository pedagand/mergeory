#![allow(clippy::large_enum_variant)]

use mrsop_codegen::syn_codegen;

syn_codegen! {
    pub(crate) mod hash {
        use crate::family_traits::Convert;
        use crate::hash_tree::{HashTagged, TreeHasher};

        #[derive(Hash, PartialEq, Eq)]
        extend_family! {
            Expr as HashTagged<Expr>,
            for<T> Vec<T> as Vec<HashTagged<T>>,

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

        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        family_impl!(Convert<syn, self> for TreeHasher);
    }

    pub(crate) mod elided {
        use crate::family_traits::{Convert, Visit};
        use crate::elided_tree::{Elider, MaybeElided, WantedElisionFinder};

        extend_family! {
            Expr as MaybeElided<Expr>,
            for<T> Vec<T> as Vec<MaybeElided<T>>,

            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Visit<super::hash> for WantedElisionFinder<'_>);
        family_impl!(Convert<super::hash, self> for Elider<'_>);
    }

    pub(crate) mod weighted {
        use crate::family_traits::Convert;
        use crate::weighted_tree::{AlignableSeq, ComputeWeight, ForgetWeight, Weighted};

        extend_family! {
            Expr as Weighted<Expr>,
            for<T> Vec<T> as AlignableSeq<T>,

            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_link!(self -> super::elided as WithoutWeight);
        family_impl!(Convert<super::elided, self> for ComputeWeight);
        family_impl!(Convert<self, super::elided> for ForgetWeight);
    }

    pub(crate) mod spine {
        use crate::family_traits::Merge;
        use crate::spine_tree::{DiffNode, SpineZipper, AlignedSeq};

        extend_family! {
            Expr as DiffNode<Expr, super::elided::Expr>,
            for<T> Vec<T> as AlignedSeq<T, super::elided::T>,

            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Merge<super::weighted, super::weighted, self> for SpineZipper);
    }

    pub mod diff {
        pub mod change {
            use crate::diff_tree::ChangeNode;
            extend_family! {
                Expr as ChangeNode<Expr>,
                for<T> Vec<T> as Vec<ChangeNode<T>>,

                proc_macro2::TokenStream as String,
                proc_macro2::Literal as String,
                proc_macro2::Span as (),
                Reserved as (),
            }
        }

        use crate::family_traits::{Convert, Visit};
        use crate::diff_tree::{AlignedSeq, DiffNode, MetavariableNamer};
        use crate::source_repr::ToSourceRepr;

        extend_family! {
            Expr as DiffNode<Expr, change::Expr>,
            for<T> Vec<T> as AlignedSeq<T, change::T>,

            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Visit<super::elided> for MetavariableNamer);
        family_impl!(Convert<super::elided, change> for MetavariableNamer);
        family_impl!(Visit<super::spine> for MetavariableNamer);
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
            #[derive(Clone)]
            extend_family! {
                Expr as InsNode<Expr>,
                for<T> Vec<T> as InsSeq<T>,

                proc_macro2::TokenStream as String,
                proc_macro2::Literal as String,
                proc_macro2::Span as (),
                Reserved as (),
            }
        }

        pub mod del {
            use crate::multi_diff_tree::DelNode;
            #[derive(Clone)]
            extend_family! {
                Expr as DelNode<Expr, super::ins::Expr>,
                for<T> Vec<T> as Vec<DelNode<T, super::ins::T>>,

                proc_macro2::TokenStream as String,
                proc_macro2::Literal as String,
                proc_macro2::Span as (),
                Reserved as (),
            }
        }

        pub(crate) mod merge_spine {
            use crate::multi_diff_tree::align_spine::{MergeSpineNode, MergeSpineSeq};
            extend_family! {
                Expr as MergeSpineNode<Expr, super::del::Expr, super::ins::Expr>,
                for<T> Vec<T> as MergeSpineSeq<T, super::del::T, super::ins::T>,

                proc_macro2::TokenStream as String,
                proc_macro2::Literal as String,
                proc_macro2::Span as (),
                Reserved as (),
            }
        }

        pub(crate) mod ins_merged_spine {
            use crate::multi_diff_tree::merge_ins::{ISpineNode, ISpineSeq};
            extend_family! {
                Expr as ISpineNode<Expr, super::del::Expr, super::ins::Expr>,
                for<T> Vec<T> as ISpineSeq<T, super::del::T, super::ins::T>,

                proc_macro2::TokenStream as String,
                proc_macro2::Literal as String,
                proc_macro2::Span as (),
                Reserved as (),
            }
        }

        use crate::multi_diff_tree::{SpineNode, SpineSeq};
        extend_family! {
            Expr as SpineNode<Expr, del::Expr, ins::Expr>,
            for<T> Vec<T> as SpineSeq<T, del::T, ins::T>,

            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_link!(ins -> del as DelEquivType);

        use crate::family_traits::{Convert, Merge, Split, Visit, VisitMut};

        use crate::multi_diff_tree::with_color::WithColor;
        family_impl!(Convert<super::diff, self> for WithColor);
        family_impl!(Convert<super::diff::change, del> for WithColor);
        family_impl!(Convert<super::diff::change, ins> for WithColor);

        use crate::multi_diff_tree::metavar_renamer::MetavarRenamer;
        family_impl!(VisitMut<del> for MetavarRenamer);
        family_impl!(VisitMut<ins> for MetavarRenamer);
        family_impl!(VisitMut<self> for MetavarRenamer);

        use crate::multi_diff_tree::align_spine::SpineAligner;
        family_impl!(Split<self, del, ins> for SpineAligner);
        family_impl!(Merge<self, self, merge_spine> for SpineAligner);
        family_impl!(Convert<self, merge_spine> for SpineAligner);

        use crate::multi_diff_tree::merge_ins::InsMerger;
        family_impl!(Merge<ins, ins, ins> for InsMerger);
        family_impl!(Merge<del, ins, del> for InsMerger);
        family_impl!(VisitMut<del> for InsMerger);
        family_impl!(Convert<merge_spine, ins_merged_spine> for InsMerger);

        use crate::multi_diff_tree::merge_del::{DelMerger, ColorAdder};
        family_impl!(Merge<del, del, del> for DelMerger);
        family_impl!(Convert<ins_merged_spine, self> for DelMerger);
        family_impl!(VisitMut<del> for ColorAdder);

        use crate::multi_diff_tree::id_merger::IdMerger;
        family_impl!(Merge<ins, ins, ins> for IdMerger);
        family_impl!(Merge<del, ins, del> for IdMerger);

        use crate::multi_diff_tree::subst::{Substituter, ColorReplacer, InferInsFromDel, SolvedConflictsRemover};
        family_impl!(VisitMut<del> for Substituter);
        family_impl!(VisitMut<ins> for Substituter);
        family_impl!(VisitMut<self> for Substituter);
        family_impl!(VisitMut<del> for ColorReplacer);
        family_impl!(Convert<del, ins> for InferInsFromDel);
        family_impl!(VisitMut<del> for SolvedConflictsRemover);
        family_impl!(VisitMut<self> for SolvedConflictsRemover);

        use crate::multi_diff_tree::metavar_remover::{MetavarRemover, InferFromSyn, InferFromSynColored};
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        family_impl!(Merge<self, syn, self> for MetavarRemover);
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        family_impl!(Merge<del, syn, del> for MetavarRemover);
        family_impl!(VisitMut<ins> for MetavarRemover);
        family_impl!(VisitMut<del> for MetavarRemover);
        family_impl!(VisitMut<self> for MetavarRemover);
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        family_impl!(Convert<syn, del> for InferFromSynColored);
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        family_impl!(Convert<syn, ins> for InferFromSyn);
        #[extra_call(proc_macro2::TokenStream, proc_macro2::Literal, proc_macro2::Span, Reserved)]
        family_impl!(Convert<syn, self> for InferFromSyn);

        use crate::multi_diff_tree::conflict_counter::ConflictCounter;
        family_impl!(Visit<del> for ConflictCounter);
        family_impl!(Visit<ins> for ConflictCounter);
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
    }
}
