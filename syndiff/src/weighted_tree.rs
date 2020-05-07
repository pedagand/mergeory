use crate::ast;
use crate::convert::Convert;
use crate::ellided_tree::MaybeEllided;

pub struct Weighted<T> {
    pub node: MaybeEllided<T>,
    pub weight: u32,
}

pub struct AlignableSeq<T>(pub Vec<Weighted<T>>);

pub struct ComputeWeight(u32);

impl<In, Out> Convert<MaybeEllided<In>, Weighted<Out>> for ComputeWeight
where
    ComputeWeight: Convert<MaybeEllided<In>, MaybeEllided<Out>>,
{
    fn convert(&mut self, input: MaybeEllided<In>) -> Weighted<Out> {
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

impl<In, Out> Convert<Weighted<In>, MaybeEllided<Out>> for ForgetWeight
where
    ForgetWeight: Convert<MaybeEllided<In>, MaybeEllided<Out>>,
{
    fn convert(&mut self, input: Weighted<In>) -> MaybeEllided<Out> {
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

macro_rules! skip_maybe_ellided {
    {$($convert_ty:ty),*} => {
        $(impl<In, Out> Convert<MaybeEllided<In>, MaybeEllided<Out>> for $convert_ty
        where
            $convert_ty: Convert<In, Out>,
        {
            fn convert(&mut self, input: MaybeEllided<In>) -> MaybeEllided<Out> {
                match input {
                    MaybeEllided::InPlace(node) => MaybeEllided::InPlace(self.convert(node)),
                    MaybeEllided::Ellided(hash) => MaybeEllided::Ellided(hash),
                }
            }
        })*
    }
}
skip_maybe_ellided!(ComputeWeight, ForgetWeight);

// We need a way to refer to a uniquely defined type without scope annotations
pub trait ForgettableWeight {
    type WithoutWeight;
}

macro_rules! impl_forgettable_weight {
    { $($weighted:ty => $nonweighted:ty,)* } => {
        $(impl ForgettableWeight for $weighted {
            type WithoutWeight = $nonweighted;
        })*
    }
}
impl_forgettable_weight! {
    ast::weighted::Expr => ast::ellided::Expr,
    ast::weighted::Item => ast::ellided::Item,
    ast::weighted::Stmt => ast::ellided::Stmt,
}

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
