use mrsop_codegen::mrsop_codegen;

mrsop_codegen! {
    pub(crate) enum TokenTree {
        Group(proc_macro2::Delimiter, Vec<TokenTree>),
        Leaf(String),
    }

    pub(crate) mod hash {
        use crate::hash_tree::HashTagged;

        #[derive(PartialEq, Eq)]
        extend_family! {
            Vec<TokenTree> as Vec<HashTagged<TokenTree>>,
        }
    }

    pub(crate) mod elided {
        use crate::family_traits::{Convert, Visit};
        use crate::elided_tree::{Elider, MaybeElided, WantedElisionFinder};

        extend_family! {
            Vec<TokenTree> as Vec<MaybeElided<TokenTree>>,
        }

        family_impl!(Visit<super::hash> for WantedElisionFinder<'_>);
        family_impl!(Convert<super::hash, self> for Elider<'_>);
    }

    pub(crate) mod weighted {
        use crate::family_traits::Convert;
        use crate::weighted_tree::{AlignableSeq, ComputeWeight, ForgetWeight};

        extend_family! {
            Vec<TokenTree> as AlignableSeq<TokenTree>,
        }

        impl crate::ast::weighted::WithoutWeight for TokenTree {
            type WithoutWeight = super::elided::TokenTree;
        }

        family_impl!(Convert<super::elided, self> for ComputeWeight);
        family_impl!(Convert<self, super::elided> for ForgetWeight);
    }

    pub(crate) mod spine {
        use crate::family_traits::Merge;
        use crate::spine_tree::{SpineZipper, AlignedSeq};

        extend_family! {
            Vec<TokenTree> as AlignedSeq<TokenTree, super::elided::TokenTree>,
        }

        family_impl!(Merge<super::weighted, super::weighted, self> for SpineZipper);
    }

    pub mod diff {
        pub mod change {
            use crate::diff_tree::ChangeNode;
            extend_family! {
                Vec<TokenTree> as Vec<ChangeNode<TokenTree>>,
            }
        }

        use crate::family_traits::{Convert, Visit};
        use crate::diff_tree::{AlignedSeq, MetavariableNamer};

        extend_family! {
            Vec<TokenTree> as AlignedSeq<TokenTree, change::TokenTree>,
        }

        family_impl!(Visit<super::elided> for MetavariableNamer);
        family_impl!(Convert<super::elided, change> for MetavariableNamer);
        family_impl!(Visit<super::spine> for MetavariableNamer);
        family_impl!(Convert<super::spine, self> for MetavariableNamer);
    }

    pub mod multi_diff {
        pub mod ins {
            use crate::multi_diff_tree::InsSeq;
            #[derive(Clone)]
            extend_family! {
                Vec<TokenTree> as InsSeq<TokenTree>,
            }
        }

        pub mod del {
            use crate::multi_diff_tree::DelNode;
            #[derive(Clone)]
            extend_family! {
                Vec<TokenTree> as Vec<DelNode<TokenTree, super::ins::TokenTree>>,
            }
        }

        pub(crate) mod merge_spine {
            use crate::multi_diff_tree::align_spine::MergeSpineSeq;
            extend_family! {
                Vec<TokenTree> as MergeSpineSeq<TokenTree, super::del::TokenTree, super::ins::TokenTree>,
            }
        }

        pub(crate) mod ins_merged_spine {
            use crate::multi_diff_tree::merge_ins::ISpineSeq;
            extend_family! {
                Vec<TokenTree> as ISpineSeq<TokenTree, super::del::TokenTree, super::ins::TokenTree>,
            }
        }

        use crate::multi_diff_tree::SpineSeq;
        extend_family! {
            Vec<TokenTree> as SpineSeq<TokenTree, del::TokenTree, ins::TokenTree>,
        }

        impl crate::ast::multi_diff::DelEquivType for ins::TokenTree {
            type DelEquivType = del::TokenTree;
        }

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

        use crate::multi_diff_tree::metavar_remover::MetavarRemover;
        family_impl!(VisitMut<ins> for MetavarRemover);
        family_impl!(VisitMut<del> for MetavarRemover);
        family_impl!(VisitMut<self> for MetavarRemover);

        use crate::multi_diff_tree::conflict_counter::ConflictCounter;
        family_impl!(Visit<del> for ConflictCounter);
        family_impl!(Visit<ins> for ConflictCounter);
        family_impl!(Visit<self> for ConflictCounter);
    }
}
