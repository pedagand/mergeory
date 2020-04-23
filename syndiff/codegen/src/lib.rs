//! A code generator for mutually recursive sum of products.
//!
//! This crates provides procedural macros to generate code for mutually
//! recursive `struct` and `enum` trees.
//!
//! It provides sevral code generation macros that can only appear inside a
//! [`mrsop_codegen!`] block that defines the mutually recursive family to work on.
//!
//! [`mrsop_codegen!`]: macro.mrsop_codegen.html

use family::Family;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput, Error, Item, ItemMod, Path, Result};

mod auto_impl;
mod extend;
mod family;
mod modgen;
mod syn_family;

/// Main macro of the crate, that declares a mutually recursive family and
/// generate code for it.
///
/// Its input should consit of a list of `struct`s, `enum`s and `mod`s.
/// The set of `struct`s and `enum`s is the recursive family on which all the
/// macros inside the generated modules will work on.
///
/// Each module is then outputed normally except that other macros of this crate
/// will work inside them.
///
/// # Example
/// ```
/// use mrsop_codegen::mrsop_codegen;
///
/// trait Convert<In, Out> {
///     fn convert(&mut self, input: In) -> Out;
/// }
///
/// mrsop_codegen! {
///     enum Foo {
///         Bar(Box<FooBar>),
///         Baz,
///     }
///     struct FooBar {
///         hello: String,
///         world: Foo,
///     }
///
///     mod length {
///         use crate::Convert;
///
///         extend_family! {
///             Box<FooBar> as usize
///         }
///
///         struct ComputeLength;
///
///         impl Convert<Box<super::FooBar>, usize> for ComputeLength {
///             fn convert(&mut self, input: Box<super::FooBar>) -> usize {
///                 input.hello.len() + match self.convert(input.world) {
///                     Foo::Bar(len) => len,
///                     Foo::Baz => 0,
///                 }
///             }
///         }
///
///         family_impl!(Convert<super, self> for ComputeLength);
///
///         pub(super) fn compute(foo: super::Foo) -> usize {
///             match ComputeLength.convert(foo) {
///                 Foo::Bar(len) => len,
///                 Foo::Baz => 0,
///             }
///         }
///     }
/// }
///
/// fn main() {
///     let tree = Foo::Bar(Box::new(FooBar{hello: "Hello".to_string(), world: Foo::Baz}));
///     assert_eq!(length::compute(tree), 5)
/// }
/// ```
///
/// # `reuse` attribute
/// You can prefix any enum or struct item by the attribute `#[reuse(path)]` to
/// prevent the type from beeing outputed and instead use an exising type.
/// The definition of the type must still be copied however.
/// ```
/// mod other_mod {
///     pub enum A {
///         I32(i32),
///         I64(i64),
///         Never,
///     }
/// }
///
/// # use mrsop_codegen::mrsop_codegen;
/// mrsop_codegen! {
///     #[reuse(other_mod::A)]
///     enum B {
///         I32(i32),
///         I64(i64),
///     }
///
///     // [...]
/// }
/// # fn main() {}
/// ```
/// If variants are omitted in the copied version, the program will panic if
/// any generated function sees them.
/// Apart from that any difference between the original type and the copy will
/// result in a compilation error.
/// Inside macros of this crate, the type should be referred by its new name
/// (`B` in the above example).
#[proc_macro]
pub fn mrsop_codegen(tokens: TokenStream) -> TokenStream {
    struct MacroInput {
        mrsop_family: Vec<DeriveInput>,
        generated_modules: Vec<ItemMod>,
    }

    impl Parse for MacroInput {
        fn parse(input: ParseStream) -> Result<Self> {
            let mut mrsop_family = Vec::new();
            let mut generated_modules = Vec::new();
            while !input.is_empty() {
                let item = input.parse()?;
                match item {
                    Item::Enum(item) => mrsop_family.push(item.into()),
                    Item::Struct(item) => mrsop_family.push(item.into()),
                    Item::Mod(item) => generated_modules.push(item),
                    _ => {
                        return Err(Error::new(
                            item.span(),
                            "Only enum, struct and mod allowed in mrsop_codegen! macro",
                        ))
                    }
                }
            }

            Ok(MacroInput {
                mrsop_family,
                generated_modules,
            })
        }
    }

    let mut input = parse_macro_input!(tokens as MacroInput);

    let input_family: Vec<_> = input
        .mrsop_family
        .iter_mut()
        .map(|item| {
            let mut replacement = None;
            item.attrs = item
                .attrs
                .drain(..)
                .filter(|attr| {
                    if attr.path.is_ident("reuse") {
                        replacement = Some(attr.parse_args::<Path>());
                        false
                    } else {
                        true
                    }
                })
                .collect();
            match replacement {
                Some(Ok(use_path)) => {
                    let vis = &item.vis;
                    let item_name = &item.ident;
                    quote!(#vis use #use_path as #item_name;)
                }
                Some(Err(err)) => err.to_compile_error(),
                None => quote!(#item),
            }
        })
        .collect();

    let mrsop_family = Family::new(&input.mrsop_family);
    let generated_modules = modgen::process_modules(input.generated_modules, &mrsop_family);

    TokenStream::from(quote! {
        #(#input_family)*
        #generated_modules
    })
}

/// Same as [`mrsop_codegen!`] but the family is the Rust AST from the [syn] crate.
///
/// Inside other macros of this crate, the syn AST types should be referred
/// without their `syn::` prefix.
///
/// [`mrsop_codegen!`]: macro.mrsop_codegen.html
/// [syn]: ../syn/index.html
#[proc_macro]
pub fn syn_codegen(tokens: TokenStream) -> TokenStream {
    struct MacroInput(Vec<ItemMod>);

    impl Parse for MacroInput {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(MacroInput({
                let mut modules = Vec::new();
                while !input.is_empty() {
                    modules.push(input.parse()?);
                }
                modules
            }))
        }
    }

    let MacroInput(input) = parse_macro_input!(tokens);

    let syn_nodes = syn_family::get_nodes();
    let syn_family = Family::new(&syn_nodes);
    let generated_modules = modgen::process_modules(input, &syn_family);

    TokenStream::from(generated_modules)
}

/// Generate an extended version of the mutually recursive family of the
/// current environment.
///
/// Extended versions are defined by a set of type replacements provided as a
/// comma separated list of `OriginalType as ReplacedType`.
///
/// Typically, to create a new version of an enum `A` with an `i32` label on
/// each element, we can write the following code:
/// ```
/// # use mrsop_codegen::mrsop_codegen;
/// mrsop_codegen! {
///     enum A {
///         Empty,
///         Another(i32, Box<A>),
///         List(Vec<A>),
///     }
///
///     mod i32_tag {
///         use std::rc::Rc;
///
///         struct Tagged<T> {
///             data: Rc<T>,
///             tag: i32,
///         }
///
///         extend_family!(Box<A> as Tagged<A>, A as Tagged<A>);
///     }
/// }
/// # fn main() {}
/// ```
/// that will produce the following module `i32_tag`:
/// ```
/// mod i32_tag {
///     use std::rc::Rc;
///
///     struct Tagged<T> {
///         data: Rc<T>,
///         tag: i32,
///     }
///
///     pub enum A {
///         Empty,
///         Another(i32, Tagged<A>),
///         List(Vec<Tagged<A>>),
///     }
/// }
/// ```
///
/// As you can see in the example above, if there is two possible replacements,
/// the outermost one is chosen.
///
/// Moreover all the attributes of the macro will be duplicated for each newly
/// generated type.
#[proc_macro]
pub fn extend_family(_: TokenStream) -> TokenStream {
    panic!("extend_family! can only be used inside mrsop_codegen! modules");
}

/// Create implementations for all members of the family for one of the
/// following trait:
///
/// * `Convert` trait that recursively transforms an instance of the recursive
/// family into a potentially different extension of the recursive family.
/// The implemented trait must have the following definition:
/// ```
/// trait Convert<In, Out> {
///     fn convert(&mut self, input: In) -> Out;
/// }
/// ```
///
/// * `Visit` trait that recursively walk through an instance of a variant of
/// the recursive family.
/// The implemented trait must have the following definition:
/// ```
/// trait Visit<T> {
///     fn visit(&mut self, input: &T);
/// }
/// ```
///
/// * `VisitMut` trait that visits a tree and potentially allows for in place
/// modifications.
/// The implemented trait must have the following definition:
/// ```
/// trait VisitMut<T> {
///     fn visit_mut(&mut self, input: &mut T);
/// }
/// ```
///
/// Sadly, Rust doesn't allow to import a trait defined by a proc-macro crate,
/// so users need to redefine Convert, Visit and VisitMut when they need to use
/// them.
///
/// The traits will be implemented for all types in the family by simply
/// performing a recursion.
/// However, you will have to manually specify how to deal with types outside
/// the family that are containing members of the family.
///
/// For `Convert`, it will turn one variant of the AST into another (or
/// transforming elements inside the same variant).
///
/// Generally the generated implementations will simply ignore or move
/// elements that have a type outside the family.
/// If you want to generate a recursive call for external types, you can add
/// an `#[extra_call(OtherType)]` attribute to force all occurences of
/// `OtherType` to also be processed by a manually supplied implementation.
///
/// Currently the syntax of `family_impl!` is one of the following (depending
/// on the generated trait implementation):
///
/// * `family_impl!(Convert<input_mod, output_mod> for T)`
/// * `family_impl!(Visit<visited_mod> for T)`
/// * `family_impl!(VisitMut<visited_mod> for T)`
///
/// where `*_mod` represent a path to the module containing the family variant
/// (e.g. `self` for a mutually recursive family generated in the current module
/// by [`extend_family!`], `super` for the family in the parent module and
/// `syn` for the Rust AST).
///
/// [`extend_family!`]: macro.extend_family.html
///
/// # Example
/// This example shows two different ways to increment every integer inside a
/// tree, one using a `Convert` implementation and the other a `VisitMut` one.
/// ```
/// trait Convert<In, Out> {
///     fn convert(&mut self, input: In) -> Out;
/// }
///
/// trait VisitMut<T> {
///     fn visit_mut(&mut self, input: &mut T);
/// }
///
/// struct Incr;
///
/// # use mrsop_codegen::mrsop_codegen;
/// mrsop_codegen! {
///     #[derive(PartialEq, Eq, Debug, Clone)]
///     pub enum A {
///         Empty,
///         Num(i32),
///         List(Vec<A>),
///     }
///
///     mod convert_incr {
///         use crate::{Convert, Incr};
///
///         impl Convert<i32, i32> for Incr {
///             fn convert(&mut self, i: i32) -> i32 {
///                 i+1
///             }
///         }
///
///         // You need to specify how to cross container types outside the family
///         impl<In, Out> Convert<Vec<In>, Vec<Out>> for Incr where Incr: Convert<In, Out> {
///             fn convert(&mut self, v: Vec<In>) -> Vec<Out> {
///                 v.into_iter().map(|elt| self.convert(elt)).collect()
///             }
///         }
///
///         // i32 is outside the family so we have to explicily request to make
///         // conversion calls for it
///         #[extra_call(i32)]
///         family_impl!(Convert<super, super> for Incr);
///     }
///
///     mod visit_mut_incr {
///         use crate::{Incr, VisitMut};
///
///         impl VisitMut<i32> for Incr {
///             fn visit_mut(&mut self, i: &mut i32) {
///                 *i = *i+1
///             }
///         }
///
///         impl<T> VisitMut<Vec<T>> for Incr where Incr: VisitMut<T> {
///             fn visit_mut(&mut self, v: &mut Vec<T>) {
///                 for elt in v {
///                     self.visit_mut(elt)
///                 }
///             }
///         }
///
///         #[extra_call(i32)]
///         family_impl!(VisitMut<super> for Incr);
///     }
/// }
///
/// fn main() {
///     let a1 = A::List(vec![A::Empty, A::Num(0), A::List(vec![A::Num(1)])]);
///     let mut a2 = a1.clone();
///     let a_incr = A::List(vec![A::Empty, A::Num(1), A::List(vec![A::Num(2)])]);
///
///     assert_eq!(Incr.convert(a1), a_incr);
///
///     Incr.visit_mut(&mut a2);
///     assert_eq!(a2, a_incr);
/// }
/// ```
#[proc_macro]
pub fn family_impl(_: TokenStream) -> TokenStream {
    panic!("family_impl! can only be used inside mrsop_codegen! modules");
}
