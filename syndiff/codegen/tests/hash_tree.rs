use mrsop_codegen::mrsop_codegen;

trait Convert<In, Out> {
    fn convert(&mut self, input: In) -> Out;
}

#[derive(Clone)]
enum Bst {
    EndOfTree,
    AstA(B),
}

#[derive(Clone)]
struct B {
    pub hello: String,
    pub world: i32,
    pub rec: Box<Bst>,
}

mrsop_codegen! {
    #[reuse(Bst)]
    enum Ast {
        EndOfTree,
        AstA(A),
    }

    #[reuse(B)]
    struct A {
        pub hello: String,
        pub world: i32,
        pub rec: Box<Ast>,
    }

    mod hash_tree {
        use crate::Convert;
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::{HashMap, DefaultHasher};
        use std::rc::Rc;

        #[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
        struct HashSum(u64);

        #[derive(Default, Debug)]
        pub struct HashTables {
            ast_table: HashMap<HashSum, Rc<Ast>>,
        }

        #[derive(Debug)]
        pub struct HashTagged<T> {
            data: Rc<T>,
            hash: HashSum,
        }

        #[derive(Hash, PartialEq, Eq, Debug)]
        extend_family! {
            Box<Ast> as HashTagged<Ast>,
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
        impl<T> Eq for HashTagged<T> { }

        impl Convert<Box<super::Ast>, HashTagged<Ast>> for HashTables {
            fn convert(&mut self, input: Box<super::Ast>) -> HashTagged<Ast> {
                let ast: Ast = self.convert(*input);
                let hash_tagged = HashTagged::from(ast);
                let existing_item = self
                    .ast_table
                    .entry(hash_tagged.hash)
                    .or_insert(hash_tagged.data.clone());
                assert!(hash_tagged.data == *existing_item);
                hash_tagged
            }
        }

        family_impl!(Convert<super, self> for HashTables);
    }
}

#[test]
fn simple() {
    let ast = Ast::AstA(A {
        hello: "hello".to_string(),
        world: 42,
        rec: Box::new(Ast::EndOfTree),
    });
    let mut hash_tables = hash_tree::HashTables::default();
    let hash_ast = hash_tables.convert(ast.clone());

    let mut hash_tables2 = hash_tree::HashTables::default();
    let hash_ast2 = hash_tables2.convert(ast);

    assert!(hash_ast == hash_ast2);
}
