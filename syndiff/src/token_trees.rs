use mrsop_codegen::mrsop_codegen;

mrsop_codegen! {
    pub enum TokenTree {
        Group(proc_macro2::Delimiter, Vec<TokenTree>),
        Leaf(String),
    }

    pub(crate) mod hash {
        use crate::family_traits::Convert;
        use crate::hash_tree::{HashTagged, TreeHasher};

        #[derive(PartialEq, Eq)]
        extend_family! {
            Vec<TokenTree> as Vec<HashTagged<TokenTree>>,
        }

        family_impl!(Convert<super, self> for TreeHasher);
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

        use crate::source_repr::ToSourceRepr;
        family_impl!(Convert<change, super> for ToSourceRepr);
        family_impl!(Convert<self, super> for ToSourceRepr);
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

        use crate::multi_diff_tree::metavar_remover::{MetavarRemover, InferFromSyn, InferFromSynColored};
        family_impl!(Merge<self, super, self> for MetavarRemover);
        family_impl!(Merge<del, super, del> for MetavarRemover);
        family_impl!(VisitMut<ins> for MetavarRemover);
        family_impl!(VisitMut<del> for MetavarRemover);
        family_impl!(VisitMut<self> for MetavarRemover);
        family_impl!(Convert<super, del> for InferFromSynColored);
        family_impl!(Convert<super, ins> for InferFromSyn);
        family_impl!(Convert<super, self> for InferFromSyn);

        use crate::multi_diff_tree::conflict_counter::ConflictCounter;
        family_impl!(Visit<del> for ConflictCounter);
        family_impl!(Visit<ins> for ConflictCounter);
        family_impl!(Visit<self> for ConflictCounter);

        use crate::source_repr::ToSourceRepr;
        family_impl!(Convert<del, super> for ToSourceRepr);
        family_impl!(Convert<ins, super> for ToSourceRepr);
        family_impl!(Convert<self, super> for ToSourceRepr);

        use crate::multi_diff_tree::patch::InsProjection;
        family_impl!(Convert<ins, super> for InsProjection);
        family_impl!(Convert<self, super> for InsProjection);
        impl crate::ast::multi_diff::SynType for super::TokenTree {
            type SynType = super::TokenTree;
        }
    }
}

impl Clone for TokenTree {
    fn clone(&self) -> TokenTree {
        match self {
            TokenTree::Group(delim, stream) => TokenTree::Group(*delim, stream.clone()),
            TokenTree::Leaf(tok_str) => TokenTree::Leaf(tok_str.clone()),
        }
    }
}

use proc_macro2::TokenStream;

pub fn iter_token_trees(tok_stream: TokenStream) -> impl Iterator<Item = TokenTree> {
    let mut token_acc = String::new();
    tok_stream.into_iter().filter_map(move |tok| match tok {
        proc_macro2::TokenTree::Group(tg) => {
            assert!(token_acc.is_empty());
            Some(TokenTree::Group(
                tg.delimiter(),
                iter_token_trees(tg.stream()).collect(),
            ))
        }
        proc_macro2::TokenTree::Punct(tp) => {
            token_acc.push(tp.as_char());
            match tp.spacing() {
                proc_macro2::Spacing::Alone => {
                    Some(TokenTree::Leaf(std::mem::take(&mut token_acc)))
                }
                proc_macro2::Spacing::Joint => None,
            }
        }
        _ => {
            token_acc.push_str(&tok.to_string());
            Some(TokenTree::Leaf(std::mem::take(&mut token_acc)))
        }
    })
}

use quote::ToTokens;

impl ToTokens for TokenTree {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.to_token_stream())
    }

    fn to_token_stream(&self) -> TokenStream {
        use std::str::FromStr;
        match self {
            TokenTree::Group(delim, tokens) => {
                proc_macro2::TokenTree::Group(proc_macro2::Group::new(
                    *delim,
                    tokens.iter().flat_map(TokenTree::to_token_stream).collect(),
                ))
                .into()
            }
            TokenTree::Leaf(tok_str) => TokenStream::from_str(&tok_str).unwrap(),
        }
    }
}

use crate::family_traits::{Convert, Merge};

impl<O, T> Convert<TokenStream, Vec<O>> for T
where
    T: Convert<TokenTree, O>,
{
    fn convert(&mut self, input: TokenStream) -> Vec<O> {
        iter_token_trees(input)
            .map(|tok| self.convert(tok))
            .collect()
    }
}

impl<I, T> Convert<Vec<I>, TokenStream> for T
where
    T: Convert<I, TokenTree>,
{
    fn convert(&mut self, input: Vec<I>) -> TokenStream {
        input
            .into_iter()
            .map(|tt| self.convert(tt))
            .flat_map(TokenTree::into_token_stream)
            .collect()
    }
}

impl<T, M> Merge<Vec<T>, TokenStream, Vec<T>> for M
where
    M: Merge<Vec<T>, Vec<TokenTree>, Vec<T>>,
{
    fn can_merge(&mut self, seq: &Vec<T>, tokens: &TokenStream) -> bool {
        let token_vec: Vec<_> = iter_token_trees(tokens.clone()).collect();
        self.can_merge(seq, &token_vec)
    }

    fn merge(&mut self, seq: Vec<T>, tokens: TokenStream) -> Vec<T> {
        let token_vec: Vec<_> = iter_token_trees(tokens).collect();
        self.merge(seq, token_vec)
    }
}
