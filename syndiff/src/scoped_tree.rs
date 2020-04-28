use crate::ast;
use crate::convert::Convert;
use crate::ellided_tree::MaybeEllided;
use crate::hash_tree::HashSum;
use im_rc::HashSet;

macro_rules! make_sets {
    { $($name:ident: $type:ty,)* } => {
        // As we are using im_rc, cloning an instance of MetavarSets is in O(1)
        #[derive(Default, Clone, PartialEq, Eq)]
        pub struct MetavarSets {
            $($name: HashSet<HashSum>,)*
        }

        pub trait HasMetavarSet: Sized {
            fn get_set(sets: &MetavarSets) -> &HashSet<HashSum>;
            fn get_set_mut(sets: &mut MetavarSets) -> &mut HashSet<HashSum>;
            fn unit_set(hash: HashSum) -> MetavarSets;
        }


        $(impl HasMetavarSet for $type {
            fn get_set(sets: &MetavarSets) -> &HashSet<HashSum> {
                &sets.$name
            }
            fn get_set_mut(sets: &mut MetavarSets) -> &mut HashSet<HashSum> {
                &mut sets.$name
            }

            fn unit_set(hash: HashSum) -> MetavarSets {
                MetavarSets {
                    $name: HashSet::unit(hash),
                    ..MetavarSets::default()
                }
            }
        })*

        fn sets_union(s1: MetavarSets, s2: MetavarSets) -> MetavarSets {
            MetavarSets {
                $($name: s1.$name.union(s2.$name),)*
            }
        }
    }
}

make_sets! {
    expr: ast::scoped::Expr,
    item: ast::scoped::Item,
    stmt: ast::scoped::Stmt,
}

pub struct MetavarScope<T> {
    pub node: T,
    pub metavars_in_scope: MetavarSets,
}

#[derive(Default)]
pub struct ComputeScopes(MetavarSets);

impl<In, Out: HasMetavarSet> Convert<MaybeEllided<In>, MaybeEllided<MetavarScope<Out>>>
    for ComputeScopes
where
    ComputeScopes: Convert<In, Out>,
{
    fn convert(&mut self, input: MaybeEllided<In>) -> MaybeEllided<MetavarScope<Out>> {
        match input {
            MaybeEllided::Ellided(h) => {
                Out::get_set_mut(&mut self.0).insert(h);
                MaybeEllided::Ellided(h)
            }
            MaybeEllided::InPlace(node) => {
                let mut metavar_scope_computation = ComputeScopes::default();
                let node = metavar_scope_computation.convert(node);
                let metavars_in_scope = metavar_scope_computation.0;
                self.0 = sets_union(self.0.clone(), metavars_in_scope.clone());
                MaybeEllided::InPlace(MetavarScope {
                    node,
                    metavars_in_scope,
                })
            }
        }
    }
}

pub struct ForgetScopes;

impl<In, Out> Convert<MaybeEllided<MetavarScope<In>>, MaybeEllided<Out>> for ForgetScopes
where
    ForgetScopes: Convert<In, Out>,
{
    fn convert(&mut self, input: MaybeEllided<MetavarScope<In>>) -> MaybeEllided<Out> {
        match input {
            MaybeEllided::Ellided(h) => MaybeEllided::Ellided(h),
            MaybeEllided::InPlace(node) => MaybeEllided::InPlace(self.convert(node.node)),
        }
    }
}

// We need a way to refer to a uniquely defined type without scope annotations
pub trait ForgettableScope {
    type WithoutScope;
}

macro_rules! impl_forgettable_scope {
    { $($scoped:ty => $nonscoped:ty,)* } => {
        $(impl ForgettableScope for $scoped {
            type WithoutScope = $nonscoped;
        })*
    }
}
impl_forgettable_scope! {
    ast::scoped::Expr => ast::ellided::Expr,
    ast::scoped::Item => ast::ellided::Item,
    ast::scoped::Stmt => ast::ellided::Stmt,
}
