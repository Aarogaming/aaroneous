//! Syntax-level rules. Comments and string literals are data, not executable code.
use std::path::{Path, PathBuf};
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};

#[derive(Debug)]
pub struct SoundnessViolation {
    pub file_path: PathBuf,
    pub line: usize,
    pub rule: &'static str,
}

pub struct SoundnessVisitor<'a> {
    pub file_path: &'a Path,
    pub violations: Vec<SoundnessViolation>,
}

impl<'ast> Visit<'ast> for SoundnessVisitor<'_> {
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if node
            .path
            .segments
            .last()
            .is_some_and(|s| matches!(s.ident.to_string().as_str(), "todo" | "unimplemented"))
        {
            self.violations.push(SoundnessViolation {
                file_path: self.file_path.into(),
                line: node.span().start().line,
                rule: "executable-stub",
            });
        }
        visit::visit_macro(self, node);
    }
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if node.unsafety.is_some()
            && node.trait_.as_ref().is_some_and(|(path, _)| {
                path.segments
                    .last()
                    .is_some_and(|s| matches!(s.ident.to_string().as_str(), "Pod" | "Zeroable"))
            })
        {
            self.violations.push(SoundnessViolation {
                file_path: self.file_path.into(),
                line: node.span().start().line,
                rule: "manual-pod-implementation",
            });
        }
        visit::visit_item_impl(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_unsafe_pod_impl() {
        let code_unsafe = ["unsafe ", "impl ", "Pod for X {}"].concat();
        let file_unsafe: syn::File = syn::parse_str(&code_unsafe).unwrap();
        let mut visitor_unsafe = SoundnessVisitor {
            file_path: Path::new("test.rs"),
            violations: Vec::new(),
        };
        visitor_unsafe.visit_file(&file_unsafe);
        assert_eq!(visitor_unsafe.violations.len(), 1);
        assert_eq!(
            visitor_unsafe.violations[0].rule,
            "manual-pod-implementation"
        );

        let code_safe = "impl Pod for X {}";
        let file_safe: syn::File = syn::parse_str(code_safe).unwrap();
        let mut visitor_safe = SoundnessVisitor {
            file_path: Path::new("test.rs"),
            violations: Vec::new(),
        };
        visitor_safe.visit_file(&file_safe);
        assert_eq!(visitor_safe.violations.len(), 0);
    }

    #[test]
    fn detects_unsafe_zeroable_impl() {
        let code_unsafe = ["unsafe ", "impl ", "Zeroable for Y {}"].concat();
        let file_unsafe: syn::File = syn::parse_str(&code_unsafe).unwrap();
        let mut visitor_unsafe = SoundnessVisitor {
            file_path: Path::new("test.rs"),
            violations: Vec::new(),
        };
        visitor_unsafe.visit_file(&file_unsafe);
        assert_eq!(visitor_unsafe.violations.len(), 1);
        assert_eq!(
            visitor_unsafe.violations[0].rule,
            "manual-pod-implementation"
        );

        let code_safe = "impl Zeroable for Y {}";
        let file_safe: syn::File = syn::parse_str(code_safe).unwrap();
        let mut visitor_safe = SoundnessVisitor {
            file_path: Path::new("test.rs"),
            violations: Vec::new(),
        };
        visitor_safe.visit_file(&file_safe);
        assert_eq!(visitor_safe.violations.len(), 0);
    }
}
