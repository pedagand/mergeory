use crate::convert::Convert;
use crate::hash_tree::{HasHashTable, HashSum, HashTables, HashTagged};
use crate::visit::Visit;
use std::rc::Rc;

/// Checks which ellisions would indeed be performed from the `possible_ellisions`
/// list and add them to the `wanted_ellisions` list.
pub struct WantedEllisionFinder<'a> {
    pub possible_ellisions: &'a HashTables,
    pub wanted_ellisions: HashTables,
}

impl WantedEllisionFinder<'_> {
    pub fn new(possible_ellisions: &HashTables) -> WantedEllisionFinder {
        WantedEllisionFinder {
            possible_ellisions,
            wanted_ellisions: HashTables::default(),
        }
    }
}

impl<'a, T: HasHashTable> Visit<HashTagged<T>> for WantedEllisionFinder<'a>
where
    WantedEllisionFinder<'a>: Visit<T>,
{
    fn visit(&mut self, input: &HashTagged<T>) {
        match T::get_table(self.possible_ellisions).get(&input.hash) {
            Some(t) => {
                // Add the replacement to the wanted ellisions and do NOT recurse
                T::get_table_mut(&mut self.wanted_ellisions).insert(input.hash, t.clone());
            }
            None => {
                // We want ellisions further down this tree as the node is not
                // ellided itself.
                self.visit(&input.data)
            }
        }
    }
}

#[derive(Debug)]
pub enum MaybeEllided<T> {
    InPlace(T),
    Ellided(HashSum),
}

pub struct Ellider<'a> {
    ellision_tables: &'a HashTables,
}

impl<'a> Ellider<'a> {
    pub fn new(ellision_tables: &HashTables) -> Ellider {
        Ellider { ellision_tables }
    }
}

impl<'a, In: HasHashTable, Out> Convert<HashTagged<In>, MaybeEllided<Out>> for Ellider<'a>
where
    Ellider<'a>: Convert<In, Out>,
{
    fn convert(&mut self, input: HashTagged<In>) -> MaybeEllided<Out> {
        if In::get_table(self.ellision_tables).contains_key(&input.hash) {
            MaybeEllided::Ellided(input.hash)
        } else {
            MaybeEllided::InPlace(self.convert(
                Rc::try_unwrap(input.data).unwrap_or_else(|_| {
                    panic!("Multiple references to a node outside hash tables")
                }),
            ))
        }
    }
}
