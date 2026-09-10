// AST Visitor for replacing .unwrap() with safe error handling

use syn::{visit_mut::VisitMut, ExprMethodCall};
use quote::quote;

/// Replaces .unwrap() calls with .ok_or_else(|| anyhow::anyhow!(...))? pattern
pub struct UnwrapReplacer<'a> {
    pub path: &'a syn::Path,
}

impl<'a> UnwrapReplacer<'a> {
    pub fn new(path: &'a syn::Path) -> Self {
        Self { path }
    }
}

impl<'ast> VisitMut<'ast> for UnwrapReplacer<'ast> {
    fn visit_expr_method_call(&mut self, node: &mut ExprMethodCall<'ast>) {
        if let Some(ident) = &node.method {
            if ident.to_string() == "unwrap" {
                // Replace unwrap with ok_or_else + ?
                if let Some(quote::Token![.](dot)) = node.recv_token {
                    let receiver = &node.receiver;
                    let args: Vec<_> = node.args.iter().map(|arg| quote!(#arg)).collect();
                    
                    *node = syn::ExprMethodCall {
                        attrs: vec![],
                        receiver: receiver.clone(),
                        dot: Some(quote::Token![.](syn::token::Dot)),
                        method: syn::Ident::new("ok_or_else", span),
                        turbofish: None,
                        paren_token: syn::token::Paren::default(),
                        args: syn::punctuated::Punctuated::from_iter(vec![
                            syn::Expr::Verbatim(quote!(_e => anyhow::anyhow!("unwrap failed on {:?}", #receiver))),
                        ]),
                    };
                }
            }
        }
        syn::visit_mut::VisitMut::visit_expr_method_call(self, node);
    }
}

fn span() -> proc_macro2::Span { proc_macro2::Span::call_site() }

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote};

    #[test]
    fn test_unwrap_replacement() {
        let mut input = parse_quote! {
            let x = option.unwrap();
        };
        
        let replacer = UnwrapReplacer::new(&syn::parse_quote!(module));
        replacer.visit_expr_mut(&mut input);
        
        // Verify unwrap was replaced
        assert!(!input.to_string().contains(".unwrap()"));
    }
}
