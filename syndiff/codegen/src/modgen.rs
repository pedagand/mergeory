use crate::family::Family;
use crate::{convert, extend};
use proc_macro2::TokenStream;
use quote::quote;
use syn::fold::Fold;
use syn::spanned::Spanned;
use syn::{Error, Item, ItemMod};

struct ModuleTransformer<'a>(&'a Family<'a>);

impl<'a> Fold for ModuleTransformer<'a> {
    fn fold_item(&mut self, item: Item) -> Item {
        match item {
            Item::Macro(item) => {
                let item = self.fold_item_macro(item);
                if item.mac.path.is_ident("extend_family") {
                    Item::Verbatim(extend::extend_family(
                        item.mac.tokens.clone(),
                        &item.attrs,
                        self.0,
                    ))
                } else if item.mac.path.is_ident("impl_convert") {
                    Item::Verbatim(convert::impl_convert(
                        item.mac.tokens.clone(),
                        &item.attrs,
                        self.0,
                    ))
                } else {
                    Item::Macro(item)
                }
            }
            _ => syn::fold::fold_item(self, item),
        }
    }
}

pub fn process_modules(modules: Vec<ItemMod>, family: &Family) -> TokenStream {
    let module_iter = modules.into_iter().map(|module| {
        if module.content.is_none() {
            return Error::new(
                module.span(),
                "Invalid mod without body inside mrsop_codegen!",
            )
            .to_compile_error();
        };
        let mut module_transformer = ModuleTransformer(&family);
        let new_module = module_transformer.fold_item_mod(module);
        quote!(#new_module)
    });
    quote!(#(#module_iter)*)
}