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
            && node.trait_.as_ref().is_some_and(|(_, path, _)| {
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
