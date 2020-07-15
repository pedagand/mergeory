use crate::family_traits::{Convert, Visit};
use crate::hash_tree::{HasHashTable, HashSum, HashTables, HashTagged};
use std::rc::Rc;

/// Checks which elisions would indeed be performed from the `possible_elisions`
/// list and add them to the `wanted_elisions` list.
pub struct WantedElisionFinder<'a> {
    possible_elisions: &'a HashTables,
    wanted_elisions: HashTables,
}

impl<'a, T: HasHashTable> Visit<HashTagged<T>> for WantedElisionFinder<'a>
where
    WantedElisionFinder<'a>: Visit<T>,
{
    fn visit(&mut self, input: &HashTagged<T>) {
        match T::get_table(self.possible_elisions).get(&input.hash) {
            Some(t) => {
                // Add the replacement to the wanted elisions and do NOT recurse
                T::get_table_mut(&mut self.wanted_elisions).insert(input.hash, t.clone());
            }
            None => {
                // We want elisions further down this tree as the node is not
                // elided itself.
                self.visit(&input.data)
            }
        }
    }
}

pub fn find_wanted_elisions<'a, T>(input: &T, possible_elisions: &'a HashTables) -> HashTables
where
    WantedElisionFinder<'a>: Visit<T>,
{
    let mut wanted_elision_finder = WantedElisionFinder {
        possible_elisions,
        wanted_elisions: HashTables::default(),
    };
    wanted_elision_finder.visit(input);
    wanted_elision_finder.wanted_elisions
}

pub enum MaybeElided<T> {
    InPlace(T),
    Elided(HashSum),
}

pub struct Elider<'a> {
    elision_tables: &'a HashTables,
}

impl<'a, In: HasHashTable, Out> Convert<HashTagged<In>, MaybeElided<Out>> for Elider<'a>
where
    Elider<'a>: Convert<In, Out>,
{
    fn convert(&mut self, input: HashTagged<In>) -> MaybeElided<Out> {
        if In::get_table(self.elision_tables).contains_key(&input.hash) {
            MaybeElided::Elided(input.hash)
        } else {
            MaybeElided::InPlace(self.convert(
                Rc::try_unwrap(input.data).unwrap_or_else(|_| {
                    panic!("Multiple references to a node outside hash tables")
                }),
            ))
        }
    }
}

pub fn elide_tree_with<'a, In, Out>(input: In, elision_tables: &'a HashTables) -> Out
where
    Elider<'a>: Convert<In, Out>,
{
    let mut elider = Elider { elision_tables };
    elider.convert(input)
}
