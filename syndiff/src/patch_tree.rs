use crate::convert::Convert;
use crate::ellided_tree::MaybeEllided;
use crate::merge::Merge;
use crate::scoped_tree::{ForgetScopes, ForgettableScope, HasMetavarSet, MetavarScope};

#[derive(Debug)]
pub enum DiffNode<Spine, Change> {
    Spine(Spine),
    Changed(MaybeEllided<Change>, MaybeEllided<Change>),
    Unchanged,
}

pub struct SpineZipper;

impl<In: HasMetavarSet + ForgettableScope, Out>
    Merge<
        MaybeEllided<MetavarScope<In>>,
        MaybeEllided<MetavarScope<In>>,
        DiffNode<Out, In::WithoutScope>,
    > for SpineZipper
where
    SpineZipper: Merge<In, In, Out>,
    ForgetScopes: Convert<MaybeEllided<MetavarScope<In>>, MaybeEllided<In::WithoutScope>>,
{
    fn can_merge(
        &mut self,
        del: &MaybeEllided<MetavarScope<In>>,
        ins: &MaybeEllided<MetavarScope<In>>,
    ) -> bool {
        // Here we must reject fusions that do not preserve the invariant
        // saying that metavariables must never cross the spine.
        // Therefore we cannot merge if the deleted metavars do not match
        // the inserted ones.

        let del_metavars = match del {
            MaybeEllided::InPlace(del_node) => del_node.metavars_in_scope.clone(),
            MaybeEllided::Ellided(del_hash) => In::unit_set(*del_hash),
        };
        let ins_metavars = match ins {
            MaybeEllided::InPlace(ins_node) => ins_node.metavars_in_scope.clone(),
            MaybeEllided::Ellided(ins_hash) => In::unit_set(*ins_hash),
        };

        del_metavars == ins_metavars
    }

    fn merge(
        &mut self,
        del: MaybeEllided<MetavarScope<In>>,
        ins: MaybeEllided<MetavarScope<In>>,
    ) -> DiffNode<Out, In::WithoutScope> {
        match (del, ins) {
            (MaybeEllided::InPlace(del), MaybeEllided::InPlace(ins))
                if self.can_merge(&del.node, &ins.node) =>
            {
                DiffNode::Spine(self.merge(del.node, ins.node))
            }
            (MaybeEllided::Ellided(hdel), MaybeEllided::Ellided(hins)) if hdel == hins => {
                DiffNode::Unchanged
            }
            (del, ins) => {
                let del = ForgetScopes.convert(del);
                let ins = ForgetScopes.convert(ins);
                DiffNode::Changed(del, ins)
            }
        }
    }
}
