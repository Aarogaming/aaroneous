use std::fmt;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{ItemStruct, Meta};

/// Error struct for memory geometry violations
#[derive(Debug, Clone)]
pub struct MemoryGeometryViolation {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub struct_name: String,
    pub reason: String,
}

impl fmt::Display for MemoryGeometryViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MemoryGeometryViolation in {}:{}:{} - struct '{}' - {}",
            self.file_path.display(),
            self.line,
            self.column,
            self.struct_name,
            self.reason
        )
    }
}

/// Visitor for checking memory geometry violations
pub struct MemoryGeometryVisitor<'a> {
    violations: Vec<MemoryGeometryViolation>,
    file_path: &'a Path,
}

impl<'a> MemoryGeometryVisitor<'a> {
    pub fn new(file_path: &'a Path) -> Self {
        Self {
            violations: Vec::new(),
            file_path,
        }
    }

    pub fn into_violations(self) -> Vec<MemoryGeometryViolation> {
        self.violations
    }
}

impl<'ast> Visit<'ast> for MemoryGeometryVisitor<'_> {
    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let mut has_pod_or_zeroable = false;

        for attr in &i.attrs {
            if let Meta::List(list) = &attr.meta {
                if list.path.is_ident("derive") {
                    let tokens = list.tokens.to_string();
                    if tokens.contains("Pod") || tokens.contains("Zeroable") {
                        has_pod_or_zeroable = true;
                        break;
                    }
                }
            }
        }

        if has_pod_or_zeroable {
            let mut has_repr_c = false;

            for attr in &i.attrs {
                if let Meta::List(list) = &attr.meta {
                    if list.path.is_ident("repr") {
                        let tokens = list.tokens.to_string();
                        if tokens.contains("C")
                            || tokens.contains("transparent")
                            || tokens.contains("packed")
                        {
                            has_repr_c = true;
                            break;
                        }
                    }
                }
            }

            if !has_repr_c {
                self.violations.push(MemoryGeometryViolation {
                    file_path: self.file_path.to_path_buf(),
                    line: i.span().start().line,
                    column: i.span().start().column,
                    struct_name: i.ident.to_string(),
                    reason: "Struct deriving Pod/Zeroable must specify #[repr(C)] for ABI memory geometry safety".to_string(),
                });
            }
        }
    }
}

pub fn audit_memory_geometry(ast: &syn::File, file_path: &Path) -> Vec<MemoryGeometryViolation> {
    let mut visitor = MemoryGeometryVisitor::new(file_path);
    visitor.visit_file(ast);
    visitor.into_violations()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_clean_repr_c_struct() {
        let code = r#"
            #[repr(C)]
            pub struct MyStruct {
                pub x: u32,
                pub y: u32,
            }
        "#;
        let ast: syn::File = syn::parse_str(code).unwrap();
        let violations = audit_memory_geometry(&ast, Path::new("test.rs"));
        assert!(violations.is_empty());
    }

    #[test]
    fn test_missing_repr_c_struct() {
        let code = r#"
            pub struct MyStruct {
                pub x: u32,
                pub y: u32,
            }
        "#;
        let ast: syn::File = syn::parse_str(code).unwrap();
        let violations = audit_memory_geometry(&ast, Path::new("test.rs"));
        assert!(violations.is_empty());
    }

    #[test]
    fn test_pod_derive_missing_repr() {
        let code = r#"
            use bytemuck::{Pod, Zeroable};
            
            #[derive(Pod)]
            pub struct MyStruct {
                pub x: u32,
                pub y: u32,
            }
        "#;
        let ast: syn::File = syn::parse_str(code).unwrap();
        let violations = audit_memory_geometry(&ast, Path::new("test.rs"));
        assert_eq!(violations.len(), 1);
        assert!(violations[0].reason.contains("Pod/Zeroable"));
        assert!(violations[0].reason.contains("repr(C)"));
    }

    #[test]
    fn test_zeroable_derive_missing_repr() {
        let code = r#"
            use bytemuck::{Pod, Zeroable};
            
            #[derive(Zeroable)]
            pub struct MyStruct {
                pub x: u32,
                pub y: u32,
            }
        "#;
        let ast: syn::File = syn::parse_str(code).unwrap();
        let violations = audit_memory_geometry(&ast, Path::new("test.rs"));
        assert_eq!(violations.len(), 1);
        assert!(violations[0].reason.contains("Pod/Zeroable"));
        assert!(violations[0].reason.contains("repr(C)"));
    }

    #[test]
    fn test_pod_derive_with_repr_c() {
        let code = r#"
            use bytemuck::{Pod, Zeroable};
            
            #[repr(C)]
            #[derive(Pod)]
            pub struct MyStruct {
                pub x: u32,
                pub y: u32,
            }
        "#;
        let ast: syn::File = syn::parse_str(code).unwrap();
        let violations = audit_memory_geometry(&ast, Path::new("test.rs"));
        assert!(violations.is_empty());
    }

    #[test]
    fn test_repr_transparent() {
        let code = r#"
            use bytemuck::{Pod, Zeroable};
            
            #[repr(transparent)]
            #[derive(Pod)]
            pub struct MyStruct(u32);
        "#;
        let ast: syn::File = syn::parse_str(code).unwrap();
        let violations = audit_memory_geometry(&ast, Path::new("test.rs"));
        assert!(violations.is_empty());
    }

    #[test]
    fn test_repr_c_with_additional_params() {
        let code = r#"
            use bytemuck::{Pod, Zeroable};
            
            #[repr(C, align(16))]
            #[derive(Pod)]
            pub struct MyStruct {
                pub x: u32,
                pub y: u32,
            }
        "#;
        let ast: syn::File = syn::parse_str(code).unwrap();
        let violations = audit_memory_geometry(&ast, Path::new("test.rs"));
        assert!(violations.is_empty());
    }
}
