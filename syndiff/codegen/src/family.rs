use proc_macro2::Ident;
use std::collections::HashSet;
use syn::visit::Visit;
use syn::{DeriveInput, Type, TypePath};

pub struct Family<'a> {
    family: &'a [DeriveInput],
    idents: HashSet<&'a Ident>,
}

impl<'a> Family<'a> {
    pub fn new(family: &[DeriveInput]) -> Family {
        Family {
            idents: family.iter().map(|item| &item.ident).collect(),
            family,
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'a, DeriveInput> {
        self.family.iter()
    }

    pub fn contains(&self, ident: &Ident) -> bool {
        self.idents.contains(ident)
    }

    /// Return true if `typ` contains any type from this recursive family
    pub fn is_inside_type(&self, typ: &Type) -> bool {
        struct CheckPathVisitor<'a> {
            checked_idents: &'a HashSet<&'a Ident>,
            found: bool,
        }

        impl<'a> Visit<'a> for CheckPathVisitor<'a> {
            fn visit_type_path(&mut self, typ: &'a TypePath) {
                match typ.path.get_ident() {
                    Some(type_name) if self.checked_idents.contains(type_name) => {
                        self.found = true;
                        return;
                    }
                    _ => (),
                }
                self.visit_path(&typ.path);
            }
        }

        let mut visitor = CheckPathVisitor {
            checked_idents: &self.idents,
            found: false,
        };
        visitor.visit_type(typ);
        visitor.found
    }
}

impl<'a> IntoIterator for &'a Family<'a> {
    type Item = &'a DeriveInput;
    type IntoIter = std::slice::Iter<'a, DeriveInput>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
