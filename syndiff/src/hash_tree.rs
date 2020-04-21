use crate::ast;
use crate::convert::Convert;
use std::collections::hash_map::{DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct HashSum(u64);

macro_rules! make_tables {
    { $($table_name:ident: $type:ty,)* } => {
        #[derive(Default)]
        pub struct HashTables {
            $($table_name: HashMap<HashSum, Rc<$type>>,)*
        }

        pub trait HasHashTable: Sized + Hash + PartialEq {
            fn get_table(hash_tables: &mut HashTables) -> &mut HashMap<HashSum, Rc<Self>>;
        }

        $(impl HasHashTable for $type {
            fn get_table(hash_tables: &mut HashTables) -> &mut HashMap<HashSum, Rc<$type>> {
                &mut hash_tables.$table_name
            }
        })*

        pub fn tables_intersection(table1: HashTables, table2: HashTables) -> HashTables {
            HashTables {
                $($table_name: table1.$table_name.into_iter().filter(|(h, v1)| {
                    match table2.$table_name.get(h) {
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
    expr_table: ast::hash_tree::Expr,
    item_table: ast::hash_tree::Item,
    stmt_table: ast::hash_tree::Stmt,
}

#[derive(Debug)]
pub struct HashTagged<T> {
    data: Rc<T>,
    hash: HashSum,
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

impl<In, Out: HasHashTable> Convert<In, HashTagged<Out>> for HashTables
where
    HashTables: Convert<In, Out>,
{
    fn convert(&mut self, input: In) -> HashTagged<Out> {
        let converted_input = self.convert(input);
        let hash_tagged = HashTagged::from(converted_input);
        let existing_item = HasHashTable::get_table(self)
            .entry(hash_tagged.hash)
            .or_insert_with(|| hash_tagged.data.clone());
        assert!(*existing_item == hash_tagged.data);
        hash_tagged
    }
}
