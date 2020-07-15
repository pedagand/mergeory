use crate::ast;
use crate::elided_tree::MaybeElided;
use crate::family_traits::Convert;

pub struct Weighted<T> {
    pub node: MaybeElided<T>,
    pub weight: u32,
}

pub struct AlignableSeq<T>(pub Vec<Weighted<T>>);

pub struct ComputeWeight(u32);

impl<In, Out> Convert<MaybeElided<In>, Weighted<Out>> for ComputeWeight
where
    ComputeWeight: Convert<MaybeElided<In>, MaybeElided<Out>>,
{
    fn convert(&mut self, input: MaybeElided<In>) -> Weighted<Out> {
        let mut sub_weigther = ComputeWeight(1);
        let node = sub_weigther.convert(input);
        self.0 += sub_weigther.0;
        Weighted {
            node,
            weight: sub_weigther.0,
        }
    }
}

impl<InSeq: IntoIterator, Out> Convert<InSeq, AlignableSeq<Out>> for ComputeWeight
where
    ComputeWeight: Convert<InSeq::Item, Weighted<Out>>,
{
    fn convert(&mut self, input: InSeq) -> AlignableSeq<Out> {
        AlignableSeq(input.into_iter().map(|elt| self.convert(elt)).collect())
    }
}

pub struct ForgetWeight;

impl<In, Out> Convert<Weighted<In>, MaybeElided<Out>> for ForgetWeight
where
    ForgetWeight: Convert<MaybeElided<In>, MaybeElided<Out>>,
{
    fn convert(&mut self, input: Weighted<In>) -> MaybeElided<Out> {
        self.convert(input.node)
    }
}

impl<In, Out> Convert<AlignableSeq<In>, Vec<Out>> for ForgetWeight
where
    ForgetWeight: Convert<Weighted<In>, Out>,
{
    fn convert(&mut self, input: AlignableSeq<In>) -> Vec<Out> {
        input.0.into_iter().map(|elt| self.convert(elt)).collect()
    }
}

macro_rules! skip_maybe_elided {
    {$($convert_ty:ty),*} => {
        $(impl<In, Out> Convert<MaybeElided<In>, MaybeElided<Out>> for $convert_ty
        where
            $convert_ty: Convert<In, Out>,
        {
            fn convert(&mut self, input: MaybeElided<In>) -> MaybeElided<Out> {
                match input {
                    MaybeElided::InPlace(node) => MaybeElided::InPlace(self.convert(node)),
                    MaybeElided::Elided(hash) => MaybeElided::Elided(hash),
                }
            }
        })*
    }
}
skip_maybe_elided!(ComputeWeight, ForgetWeight);

// We need a way to refer to a uniquely defined type without scope annotations
pub trait ForgettableWeight {
    type WithoutWeight;
}

macro_rules! impl_forgettable_weight {
    { $($ast_typ:ident),* } => {
        $(impl ForgettableWeight for ast::weighted::$ast_typ {
            type WithoutWeight = ast::elided::$ast_typ;
        })*
    }
}
impl_forgettable_weight!(Expr, Stmt, Item, TraitItem, ImplItem, ForeignItem);

pub fn compute_weight<In, Out>(input: In) -> Out
where
    ComputeWeight: Convert<In, Out>,
{
    ComputeWeight(0).convert(input)
}

pub fn forget_weight<In, Out>(input: In) -> Out
where
    ForgetWeight: Convert<In, Out>,
{
    ForgetWeight.convert(input)
}
