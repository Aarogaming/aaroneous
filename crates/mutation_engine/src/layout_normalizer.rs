// AST Visitor for adding #[repr(C)] to unaligned structs

use syn::{visit_mut::VisitMut, DataStruct, Attribute};
use quote::quote;

/// Adds #[repr(C)] attribute to structs that lack it
pub struct LayoutNormalizer {
    pub added_repr_c: Vec<syn::Path>,
}

impl Default for LayoutNormalizer {
    fn default() -> Self {
        Self { 
            added_repr_c: Vec::new(),
        }
    }
}

impl LayoutNormalizer {
    pub fn needs_repr_c(&self, attrs: &[Attribute]) -> bool {
        !attrs.iter().any(|attr| {
            attr.path().is_ident("repr") && 
            attr.parse_args::<syn::Path>().ok()
                .map(|p| p.is_ident("C"))
                .unwrap_or(false)
        })
    }

    pub fn add_repr_c(&mut self, path: syn::Path) {
        self.added_repr_c.push(path);
    }
}

impl VisitMut for LayoutNormalizer {
    fn visit_data_struct(&mut self, node: &mut DataStruct) {
        if self.needs_repr_c(&node.attrs) {
            let repr_c_attr = Attribute {
                pound_token: Default::default(),
                style: Default::default(),
                path: syn::Path::from(syn::Ident::new("repr", proc_macro2::Span::call_site())),
                bracket_token: Default::default(),
                tokens: quote!(C).into(),
            };
            
            node.attrs.insert(0, repr_c_attr);
        }
        
        syn::visit_mut::VisitMut::visit_data_struct(self, node);
    }
}

fn span() -> proc_macro2::Span { proc_macro2::Span::call_site() }

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote};

    #[test]
    fn test_layout_normalizer() {
        let mut input = parse_quote! {
            pub struct Data {
                pub id: u8,
            }
        };
        
        let mut normalizer = LayoutNormalizer::default();
        normalizer.visit_item_mut(&mut input);
        
        // Verify #[repr(C)] was added
        let attrs = &input.attrs;
        assert!(attrs.iter().any(|attr| attr.path().is_ident("repr")));
    }
}
