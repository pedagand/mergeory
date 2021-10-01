#![allow(clippy::large_enum_variant)]

use mrsop_codegen::syn_codegen;

syn_codegen! {
    pub(crate) mod hash {
        use crate::family_traits::Convert;
        use crate::hash_tree::{HashTagged, TreeHasher};

        #[derive(Hash, PartialEq, Eq)]
        extend_family! {
            Expr as HashTagged<Expr>,
            Vec<Stmt> as Vec<HashTagged<Stmt>>,
            Vec<Item> as Vec<HashTagged<Item>>,
            Vec<TraitItem> as Vec<HashTagged<TraitItem>>,
            Vec<ImplItem> as Vec<HashTagged<ImplItem>>,
            Vec<ForeignItem> as Vec<HashTagged<ForeignItem>>,
            Vec<Attribute> as Vec<HashTagged<Attribute>>,

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
            Vec<Stmt> as Vec<MaybeElided<Stmt>>,
            Vec<Item> as Vec<MaybeElided<Item>>,
            Vec<TraitItem> as Vec<MaybeElided<TraitItem>>,
            Vec<ImplItem> as Vec<MaybeElided<ImplItem>>,
            Vec<ForeignItem> as Vec<MaybeElided<ForeignItem>>,
            Vec<Attribute> as Vec<MaybeElided<Attribute>>,

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
            Vec<Stmt> as AlignableSeq<Stmt>,
            Vec<Item> as AlignableSeq<Item>,
            Vec<TraitItem> as AlignableSeq<TraitItem>,
            Vec<ImplItem> as AlignableSeq<ImplItem>,
            Vec<ForeignItem> as AlignableSeq<ForeignItem>,
            Vec<Attribute> as AlignableSeq<Attribute>,

            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
        }

        family_impl!(Convert<super::elided, self> for ComputeWeight);
        family_impl!(Convert<self, super::elided> for ForgetWeight);
    }

    pub(crate) mod spine {
        use crate::family_traits::Merge;
        use crate::spine_tree::{DiffNode, SpineZipper, AlignedSeq};

        extend_family! {
            Expr as DiffNode<Expr, super::elided::Expr>,
            Vec<Stmt> as AlignedSeq<Stmt, super::elided::Stmt>,
            Vec<Item> as AlignedSeq<Item, super::elided::Item>,
            Vec<TraitItem> as AlignedSeq<TraitItem, super::elided::TraitItem>,
            Vec<ImplItem> as AlignedSeq<ImplItem, super::elided::ImplItem>,
            Vec<ForeignItem> as AlignedSeq<ForeignItem, super::elided::ForeignItem>,
            Vec<Attribute> as AlignedSeq<Attribute, super::elided::Attribute>,

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
                Vec<Stmt> as Vec<ChangeNode<Stmt>>,
                Vec<Item> as Vec<ChangeNode<Item>>,
                Vec<TraitItem> as Vec<ChangeNode<TraitItem>>,
                Vec<ImplItem> as Vec<ChangeNode<ImplItem>>,
                Vec<ForeignItem> as Vec<ChangeNode<ForeignItem>>,
                Vec<Attribute> as Vec<ChangeNode<Attribute>>,

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
            Vec<Stmt> as AlignedSeq<Stmt, change::Stmt>,
            Vec<Item> as AlignedSeq<Item, change::Item>,
            Vec<TraitItem> as AlignedSeq<TraitItem, change::TraitItem>,
            Vec<ImplItem> as AlignedSeq<ImplItem, change::ImplItem>,
            Vec<ForeignItem> as AlignedSeq<ForeignItem, change::ForeignItem>,
            Vec<Attribute> as AlignedSeq<Attribute, change::Attribute>,

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
                Vec<Stmt> as InsSeq<Stmt>,
                Vec<Item> as InsSeq<Item>,
                Vec<TraitItem> as InsSeq<TraitItem>,
                Vec<ImplItem> as InsSeq<ImplItem>,
                Vec<ForeignItem> as InsSeq<ForeignItem>,
                Vec<Attribute> as InsSeq<Attribute>,

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
                Vec<Stmt> as Vec<DelNode<Stmt, super::ins::Stmt>>,
                Vec<Item> as Vec<DelNode<Item, super::ins::Item>>,
                Vec<TraitItem> as Vec<DelNode<TraitItem, super::ins::TraitItem>>,
                Vec<ImplItem> as Vec<DelNode<ImplItem, super::ins::ImplItem>>,
                Vec<ForeignItem> as Vec<DelNode<ForeignItem, super::ins::ForeignItem>>,
                Vec<Attribute> as Vec<DelNode<Attribute, super::ins::Attribute>>,

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
                Vec<Stmt> as MergeSpineSeq<Stmt, super::del::Stmt, super::ins::Stmt>,
                Vec<Item> as MergeSpineSeq<Item, super::del::Item, super::ins::Item>,
                Vec<TraitItem> as MergeSpineSeq<TraitItem, super::del::TraitItem, super::ins::TraitItem>,
                Vec<ImplItem> as MergeSpineSeq<ImplItem, super::del::ImplItem, super::ins::ImplItem>,
                Vec<ForeignItem> as MergeSpineSeq<ForeignItem, super::del::ForeignItem, super::ins::ForeignItem>,
                Vec<Attribute> as MergeSpineSeq<Attribute, super::del::Attribute, super::ins::Attribute>,

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
                Vec<Stmt> as ISpineSeq<Stmt, super::del::Stmt, super::ins::Stmt>,
                Vec<Item> as ISpineSeq<Item, super::del::Item, super::ins::Item>,
                Vec<TraitItem> as ISpineSeq<TraitItem, super::del::TraitItem, super::ins::TraitItem>,
                Vec<ImplItem> as ISpineSeq<ImplItem, super::del::ImplItem, super::ins::ImplItem>,
                Vec<ForeignItem> as ISpineSeq<ForeignItem, super::del::ForeignItem, super::ins::ForeignItem>,
                Vec<Attribute> as ISpineSeq<Attribute, super::del::Attribute, super::ins::Attribute>,

                proc_macro2::TokenStream as String,
                proc_macro2::Literal as String,
                proc_macro2::Span as (),
                Reserved as (),
            }
        }

        use crate::multi_diff_tree::{SpineNode, SpineSeq};
        extend_family! {
            Expr as SpineNode<Expr, del::Expr, ins::Expr>,
            Vec<Stmt> as SpineSeq<Stmt, del::Stmt, ins::Stmt>,
            Vec<Item> as SpineSeq<Item, del::Item, ins::Item>,
            Vec<TraitItem> as SpineSeq<TraitItem, del::TraitItem, ins::TraitItem>,
            Vec<ImplItem> as SpineSeq<ImplItem, del::ImplItem, ins::ImplItem>,
            Vec<ForeignItem> as SpineSeq<ForeignItem, del::ForeignItem, ins::ForeignItem>,
            Vec<Attribute> as SpineSeq<Attribute, del::Attribute, ins::Attribute>,

            proc_macro2::TokenStream as String,
            proc_macro2::Literal as String,
            proc_macro2::Span as (),
            Reserved as (),
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
