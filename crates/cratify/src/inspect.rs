// src/inspect.rs
//! AST inspection utilities for Cratify.
//! Parses Rust source files using `syn` and extracts structural information
//! such as function signatures, struct definitions, and enum variants.
//! The results are plain data structures suitable for zero‑copy contracts
//! (they implement `Clone` and can be serialized if needed).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use syn::{File, Item, Signature, Type, Visibility};

/// Simple representation of a function signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub visibility: String,
    pub inputs: Vec<String>,
    pub output: Option<String>,
    /// Stringified function body for pattern-level audit scanning.
    pub body: String,
}

/// Simple representation of a struct field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub ty: String,
    pub visibility: String,
}

/// Simple representation of a struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructInfo {
    pub name: String,
    pub visibility: String,
    pub fields: Vec<StructField>,
}

/// Simple representation of an enum variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub payload: Option<String>,
}

/// Simple representation of an enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumInfo {
    pub name: String,
    pub visibility: String,
    pub variants: Vec<EnumVariant>,
}

/// Simple representation of a trait definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitInfo {
    pub name: String,
    pub visibility: String,
    pub supertraits: Vec<String>,
    pub methods: Vec<String>,
}

/// Simple representation of an impl block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplInfo {
    pub self_type: String,
    pub trait_name: Option<String>,
    pub methods: Vec<String>,
}

/// Simple representation of a type alias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAliasInfo {
    pub name: String,
    pub visibility: String,
    pub ty: String,
}

/// Simple representation of a const item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstInfo {
    pub name: String,
    pub visibility: String,
    pub ty: String,
}

/// Simple representation of a static item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticInfo {
    pub name: String,
    pub visibility: String,
    pub ty: String,
    pub is_mut: bool,
}

/// Aggregate of extracted items from a source file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeInfo {
    pub functions: Vec<FunctionInfo>,
    pub structs: Vec<StructInfo>,
    pub enums: Vec<EnumInfo>,
    pub traits: Vec<TraitInfo>,
    pub impls: Vec<ImplInfo>,
    pub type_aliases: Vec<TypeAliasInfo>,
    pub consts: Vec<ConstInfo>,
    pub statics: Vec<StaticInfo>,
}

impl CodeInfo {
    /// Convenience method to pretty‑print a summary.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("Functions: {}\n", self.functions.len()));
        s.push_str(&format!("Structs: {}\n", self.structs.len()));
        s.push_str(&format!("Enums: {}\n", self.enums.len()));
        s.push_str(&format!("Traits: {}\n", self.traits.len()));
        s.push_str(&format!("Impls: {}\n", self.impls.len()));
        s.push_str(&format!("TypeAliases: {}\n", self.type_aliases.len()));
        s.push_str(&format!("Consts: {}\n", self.consts.len()));
        s.push_str(&format!("Statics: {}\n", self.statics.len()));
        s
    }
}

/// Inspect a single Rust source file and return its `CodeInfo`.
pub fn inspect_code<P: AsRef<Path>>(path: P) -> Result<CodeInfo> {
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read source file {:?}", path.as_ref()))?;
    let syntax: File = syn::parse_file(&raw)
        .with_context(|| format!("Failed to parse Rust file {:?}", path.as_ref()))?;

    let mut info = CodeInfo::default();

    for item in syntax.items {
        inspect_item(item, &mut info);
    }
    Ok(info)
}

/// Recursively inspect a single `syn::Item`, dispatching to the appropriate
/// extractor and recursing into nested `mod` blocks.
fn inspect_item(item: Item, info: &mut CodeInfo) {
    match item {
        Item::Fn(func) => {
            let sig: Signature = func.sig;
            let name = sig.ident.to_string();
            let visibility = format_visibility(&func.vis);
            let inputs = sig
                .inputs
                .iter()
                .map(|arg| match arg {
                    syn::FnArg::Receiver(_) => "self".to_string(),
                    syn::FnArg::Typed(pat_type) => {
                        let ty = type_to_string(&*pat_type.ty);
                        let pat = pat_type.pat.clone();
                        let pat_str = quote::quote!(#pat).to_string();
                        format!("{}: {}", pat_str, ty)
                    }
                })
                .collect();
            let output = match &sig.output {
                syn::ReturnType::Default => None,
                syn::ReturnType::Type(_, ty) => Some(type_to_string(&*ty)),
            };
            let block = &func.block;
            let body = quote::quote!(#block).to_string();
            info.functions.push(FunctionInfo {
                name,
                visibility,
                inputs,
                output,
                body,
            });
        }
        Item::Struct(st) => {
            let name = st.ident.to_string();
            let visibility = format_visibility(&st.vis);
            let mut fields = Vec::new();
            for f in st.fields {
                let vis = format_visibility(&f.vis);
                let field_name = f.ident.map(|i| i.to_string()).unwrap_or("_".to_string());
                let ty = type_to_string(&f.ty);
                fields.push(StructField { name: field_name, ty, visibility: vis });
            }
            info.structs.push(StructInfo { name, visibility, fields });
        }
        Item::Enum(en) => {
            let name = en.ident.to_string();
            let visibility = format_visibility(&en.vis);
            let mut variants = Vec::new();
            for v in en.variants {
                let variant_name = v.ident.to_string();
                let payload = match v.fields {
                    syn::Fields::Unnamed(ref u) if !u.unnamed.is_empty() => {
                        let types: Vec<String> = u.unnamed.iter().map(|f| type_to_string(&f.ty)).collect();
                        Some(types.join(", "))
                    }
                    syn::Fields::Named(ref named) if !named.named.is_empty() => {
                        let fields: Vec<String> = named.named.iter().map(|f| {
                            let ty = type_to_string(&f.ty);
                            let name = f.ident.as_ref().unwrap().to_string();
                            format!("{}: {}", name, ty)
                        }).collect();
                        Some(fields.join(", "))
                    }
                    _ => None,
                };
                variants.push(EnumVariant { name: variant_name, payload });
            }
            info.enums.push(EnumInfo { name, visibility, variants });
        }
        Item::Mod(module) => {
            // Recurse into inline module bodies (`mod foo { ... }`)
            if let Some((_, items)) = module.content {
                for item in items {
                    inspect_item(item, info);
                }
            }
        }
        Item::Trait(trait_def) => {
            let name = trait_def.ident.to_string();
            let visibility = format_visibility(&trait_def.vis);
            let supertraits: Vec<String> = trait_def
                .supertraits
                .iter()
                .map(|bound| quote::quote!(#bound).to_string())
                .collect();
            let methods: Vec<String> = trait_def
                .items
                .iter()
                .filter_map(|item| {
                    if let syn::TraitItem::Fn(method) = item {
                        let sig_str = quote::quote!(#method.sig).to_string();
                        Some(sig_str)
                    } else {
                        None
                    }
                })
                .collect();
            info.traits.push(TraitInfo {
                name,
                visibility,
                supertraits,
                methods,
            });
        }
        Item::Impl(impl_block) => {
            let self_type = type_to_string(&impl_block.self_ty);
            let trait_name = impl_block.trait_.as_ref().map(|(_, path, _)| {
                path.segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default()
            });
            let methods: Vec<String> = impl_block
                .items
                .iter()
                .filter_map(|item| {
                    if let syn::ImplItem::Fn(method) = item {
                        let sig_str = quote::quote!(#method.sig).to_string();
                        Some(sig_str)
                    } else {
                        None
                    }
                })
                .collect();
            info.impls.push(ImplInfo {
                self_type,
                trait_name,
                methods,
            });
        }
        Item::Type(type_alias) => {
            let name = type_alias.ident.to_string();
            let visibility = format_visibility(&type_alias.vis);
            let ty = type_to_string(&*type_alias.ty);
            info.type_aliases.push(TypeAliasInfo {
                name,
                visibility,
                ty,
            });
        }
        Item::Const(const_item) => {
            let name = const_item.ident.to_string();
            let visibility = format_visibility(&const_item.vis);
            let ty = type_to_string(&*const_item.ty);
            info.consts.push(ConstInfo {
                name,
                visibility,
                ty,
            });
        }
        Item::Static(static_item) => {
            let name = static_item.ident.to_string();
            let visibility = format_visibility(&static_item.vis);
            let ty = type_to_string(&*static_item.ty);
            let is_mut = matches!(static_item.mutability, syn::StaticMutability::Mut(_));
            info.statics.push(StaticInfo {
                name,
                visibility,
                ty,
                is_mut,
            });
        }
        _ => {}
    }
}

/// Format a `syn::Visibility` into a human-readable string that preserves
/// restricted visibility qualifiers (`pub(crate)`, `pub(super)`, etc.).
fn format_visibility(vis: &Visibility) -> String {
    match vis {
        Visibility::Public(_) => "pub".to_string(),
        Visibility::Restricted(restricted) => quote::quote!(#restricted).to_string(),
        Visibility::Inherited => "private".to_string(),
    }
}

fn type_to_string(ty: &Type) -> String {
    quote::quote!(#ty).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn parses_simple_file() {
        let mut file = NamedTempFile::new().expect("tmp file");
        writeln!(file, "pub struct Foo {{ pub x: i32, y: String }}\npub enum Bar {{ A, B(i32), C {{ name: String }} }}\npub fn baz(a: i32) -> bool {{ a > 0 }}")
            .unwrap();
        let info = inspect_code(file.path()).expect("inspect");
        assert_eq!(info.structs.len(), 1);
        assert_eq!(info.enums.len(), 1);
        assert_eq!(info.functions.len(), 1);
    }
}
