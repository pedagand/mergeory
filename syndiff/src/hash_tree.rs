use crate::ast;
use crate::convert::Convert;
use std::collections::hash_map::{DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct HashSum(u64);

impl std::fmt::Display for HashSum {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

macro_rules! make_tables {
    { $($name:ident: $type:ty,)* } => {
        #[derive(Default)]
        pub struct HashTables {
            $($name: HashMap<HashSum, Rc<$type>>,)*
        }

        pub trait HasHashTable: Sized + Hash + PartialEq {
            fn get_table(hash_tables: &HashTables) -> &HashMap<HashSum, Rc<Self>>;
            fn get_table_mut(hash_tables: &mut HashTables) -> &mut HashMap<HashSum, Rc<Self>>;
        }

        $(impl HasHashTable for $type {
            fn get_table(hash_tables: &HashTables) -> &HashMap<HashSum, Rc<$type>> {
                &hash_tables.$name
            }
            fn get_table_mut(hash_tables: &mut HashTables) -> &mut HashMap<HashSum, Rc<$type>> {
                &mut hash_tables.$name
            }
        })*

        pub fn tables_intersection(table1: HashTables, table2: HashTables) -> HashTables {
            HashTables {
                $($name: table1.$name.into_iter().filter(|(h, v1)| {
                    match table2.$name.get(h) {
                        Some(v2) => {
                            assert!(v1 == v2);
                            true
                        }
                        None => false,
                    }
                }).collect(),)*
            }
        }
    }
}

make_tables! {
    expr: ast::hash::Expr,
    item: ast::hash::Item,
    stmt: ast::hash::Stmt,
}

#[derive(Debug)]
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

pub struct TreeHasher(HashTables);

impl<In, Out: HasHashTable> Convert<In, HashTagged<Out>> for TreeHasher
where
    TreeHasher: Convert<In, Out>,
{
    fn convert(&mut self, input: In) -> HashTagged<Out> {
        let converted_input = self.convert(input);
        let hash_tagged = HashTagged::from(converted_input);
        let existing_item = Out::get_table_mut(&mut self.0)
            .entry(hash_tagged.hash)
            .or_insert_with(|| hash_tagged.data.clone());
        assert!(*existing_item == hash_tagged.data);
        hash_tagged
    }
}

pub fn hash_tree<In, Out>(input: In) -> (Out, HashTables)
where
    TreeHasher: Convert<In, Out>,
{
    let mut tree_hasher = TreeHasher(HashTables::default());
    let hashed_tree = tree_hasher.convert(input);
    (hashed_tree, tree_hasher.0)
}
