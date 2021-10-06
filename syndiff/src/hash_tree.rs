use crate::family_traits::Convert;
use std::any::Any;
use std::collections::hash_map::{DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct HashSum(u64);

pub type HashTable = HashMap<HashSum, Rc<dyn Any>>;

pub fn tables_intersection(table1: HashTable, table2: HashTable) -> HashTable {
    table1
        .into_iter()
        .filter(|(h, v1)| match table2.get(h) {
            Some(v2) => {
                assert!(v1.type_id() == v2.type_id());
                true
            }
            None => false,
        })
        .collect()
}

pub struct HashTagged<T> {
    pub data: Rc<T>,
    pub hash: HashSum,
}

impl<T: Hash> From<T> for HashTagged<T> {
    fn from(data: T) -> HashTagged<T> {
        HashTagged {
            hash: {
                let mut hasher = DefaultHasher::new();
                data.hash(&mut hasher);
                HashSum(hasher.finish())
            },
            data: Rc::new(data),
        }
    }
}

impl<T> Hash for HashTagged<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash.0)
    }
}

impl<T> PartialEq for HashTagged<T> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}
impl<T> Eq for HashTagged<T> {}

pub struct TreeHasher(HashTable);

impl<In, Out> Convert<In, HashTagged<Out>> for TreeHasher
where
    TreeHasher: Convert<In, Out>,
    Out: Hash + PartialEq + 'static,
{
    fn convert(&mut self, input: In) -> HashTagged<Out> {
        let converted_input = self.convert(input);
        let hash_tagged = HashTagged::from(converted_input);
        let existing_item = self
            .0
            .entry(hash_tagged.hash)
            .or_insert_with(|| hash_tagged.data.clone());
        assert!(existing_item.downcast_ref::<Out>().unwrap() == &*hash_tagged.data);
        hash_tagged
    }
}

pub fn hash_tree<In, Out>(input: In) -> (Out, HashTable)
where
    TreeHasher: Convert<In, Out>,
{
    let mut tree_hasher = TreeHasher(HashTable::default());
    let hashed_tree = tree_hasher.convert(input);
    (hashed_tree, tree_hasher.0)
}
