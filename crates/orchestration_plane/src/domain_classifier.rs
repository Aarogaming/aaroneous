//! Domain Classifier & Signature Extraction Engine
//!
//! Inspects external Rust ASTs via `syn` and classifies modules/functions
//! into workspace domains based on semantic and structural signatures.
//!
//! Provides zero-ambient authority AST risk detection and structured ingestion reports.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    ExprCall, ExprMethodCall, File, FnArg, ImplItemFn, ItemConst, ItemEnum, ItemFn, ItemStatic,
    ItemStruct, ItemTrait, ItemType, ItemUse, Path as SynPath, ReceiverKind, ReturnType, Type,
    UseGroup, UsePath, UseRename, UseTree, Visibility,
};

/// Workspace architectural domain classification target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    /// High frequency of tensor operations, SIMD, matrix multiplication, activation functions, linear algebra, quantization, or raw pointer math.
    Compute,
    /// Kernel supervisory routines, lifecycle management, thread affinity, memory-mapped rings, CPU scheduling, or OS boundary isolation.
    Hypervisor,
    /// Cross-process communication, lock-free queues, ring buffers, shared memory, serialization over channels, or SWMR SHM.
    IpcBus,
    /// Agent workflows, execution plans, task DAGs, state machine transitions, or event-driven coordinators.
    Orchestrator,
    /// Candidates whose maximum affinity falls below the classification threshold.
    Novel(String),
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Domain::Compute => write!(f, "compute"),
            Domain::Hypervisor => write!(f, "hypervisor"),
            Domain::IpcBus => write!(f, "ipc_bus"),
            Domain::Orchestrator => write!(f, "orchestrator"),
            Domain::Novel(name) => write!(f, "novel:{name}"),
        }
    }
}

/// Extracted public item signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicItem {
    pub kind: ItemKind,
    pub name: String,
    pub signature: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    Struct,
    Trait,
    Function,
    Enum,
    TypeAlias,
    Constant,
    Static,
}

/// Ambient authority risk site flagged during AST traversal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmbientRiskSite {
    pub line: usize,
    pub column: usize,
    pub symbol: String,
    pub remediation: String,
}

/// Structured summary of AST inspection for a module or file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionReport {
    /// Path or module name analyzed.
    pub target: String,
    /// Suggested target domain.
    pub target_domain: Domain,
    /// Affinity scores per existing domain.
    pub affinity_scores: HashMap<String, f32>,
    /// Public interface items extracted.
    pub public_items: Vec<PublicItem>,
    /// External crates and dependencies detected from `use` and path expressions.
    pub detected_dependencies: HashSet<String>,
    /// Ambient authority risk sites requiring transformation before ingestion.
    pub ambient_risks: Vec<AmbientRiskSite>,
    /// Recommended ambient AST transformations.
    pub required_transformations: Vec<String>,
}

/// Configuration options for the `DomainClassifier`.
#[derive(Debug, Clone)]
pub struct ClassifierConfig {
    /// Threshold score below which a module is classified as `Domain::Novel`.
    pub novel_threshold: f32,
    /// Default candidate label for novel domains.
    pub novel_label: String,
}

impl Default for ClassifierConfig {
    fn default() -> Self {
        Self {
            novel_threshold: 0.60,
            novel_label: "unclassified_kernel".to_string(),
        }
    }
}

/// AST-based Domain Classifier and Signature Extraction Engine.
pub struct DomainClassifier {
    config: ClassifierConfig,
}

impl Default for DomainClassifier {
    fn default() -> Self {
        Self::new(ClassifierConfig::default())
    }
}

impl DomainClassifier {
    pub fn new(config: ClassifierConfig) -> Self {
        Self { config }
    }

    /// Classify a Rust source code string and produce an `IngestionReport`.
    pub fn classify_source(
        &self,
        target_name: &str,
        source: &str,
    ) -> Result<IngestionReport, syn::Error> {
        let syntax_tree: File = syn::parse_str(source)?;
        Ok(self.analyze_ast(target_name, &syntax_tree))
    }

    /// Classify an already parsed `syn::File`.
    pub fn analyze_ast(&self, target_name: &str, syntax_tree: &File) -> IngestionReport {
        let mut visitor = AstInspectionVisitor::default();
        visitor.visit_file(syntax_tree);

        let affinity_scores = self.calculate_domain_affinities(&visitor);

        let mut best_domain = Domain::Novel(self.config.novel_label.clone());
        let mut highest_score = 0.0f32;

        let domain_map = [
            ("compute", Domain::Compute),
            ("hypervisor", Domain::Hypervisor),
            ("ipc_bus", Domain::IpcBus),
            ("orchestrator", Domain::Orchestrator),
        ];

        for (domain_key, domain_variant) in &domain_map {
            if let Some(&score) = affinity_scores.get(*domain_key)
                && score > highest_score
            {
                highest_score = score;
                best_domain = domain_variant.clone();
            }
        }

        if highest_score < self.config.novel_threshold {
            let label = if visitor.total_tokens_matched == 0 {
                "unclassified_kernel".to_string()
            } else {
                format!(
                    "{}_candidate",
                    target_name.to_lowercase().replace(".rs", "")
                )
            };
            best_domain = Domain::Novel(label);
        }

        let mut required_transformations = Vec::new();
        if !visitor.ambient_risks.is_empty() {
            required_transformations.push(
                "Refactor direct `std::env` and ambient path queries into explicit typed configuration struct injection to uphold Zero-Ambient-Authority."
                    .to_string(),
            );
        }
        if visitor.raw_panic_count > 0 {
            required_transformations.push(
                "Replace panicking stubs (`panic!`, `unwrap`, `expect`) with deterministic `Result<T, E>` error propagation."
                    .to_string(),
            );
        }
        if visitor.has_unsafe_allocations {
            required_transformations.push(
                "Verify zero heap allocations on hot-path execution routines; convert to static ring buffers or stack slices."
                    .to_string(),
            );
        }

        IngestionReport {
            target: target_name.to_string(),
            target_domain: best_domain,
            affinity_scores,
            public_items: visitor.public_items,
            detected_dependencies: visitor.detected_dependencies,
            ambient_risks: visitor.ambient_risks,
            required_transformations,
        }
    }

    /// Calculate normalized affinity scores across workspace domains.
    fn calculate_domain_affinities(&self, visitor: &AstInspectionVisitor) -> HashMap<String, f32> {
        let compute_raw = visitor.domain_hits.get("compute").copied().unwrap_or(0) as f32;
        let hypervisor_raw = visitor.domain_hits.get("hypervisor").copied().unwrap_or(0) as f32;
        let ipc_bus_raw = visitor.domain_hits.get("ipc_bus").copied().unwrap_or(0) as f32;
        let orchestrator_raw = visitor
            .domain_hits
            .get("orchestrator")
            .copied()
            .unwrap_or(0) as f32;

        let total_hits = compute_raw + hypervisor_raw + ipc_bus_raw + orchestrator_raw;

        let mut scores = HashMap::new();

        if total_hits > 0.0 {
            let confidence = (total_hits / 3.0).min(1.0);

            scores.insert(
                "compute".to_string(),
                (compute_raw / total_hits) * confidence,
            );
            scores.insert(
                "hypervisor".to_string(),
                (hypervisor_raw / total_hits) * confidence,
            );
            scores.insert(
                "ipc_bus".to_string(),
                (ipc_bus_raw / total_hits) * confidence,
            );
            scores.insert(
                "orchestrator".to_string(),
                (orchestrator_raw / total_hits) * confidence,
            );
        } else {
            scores.insert("compute".to_string(), 0.0);
            scores.insert("hypervisor".to_string(), 0.0);
            scores.insert("ipc_bus".to_string(), 0.0);
            scores.insert("orchestrator".to_string(), 0.0);
        }

        scores
    }
}

/// Visitor that extracts signatures, flags ambient risks, and tallies domain semantics.
#[derive(Default)]
struct AstInspectionVisitor {
    public_items: Vec<PublicItem>,
    detected_dependencies: HashSet<String>,
    ambient_risks: Vec<AmbientRiskSite>,
    domain_hits: HashMap<&'static str, usize>,
    total_tokens_matched: usize,
    raw_panic_count: usize,
    has_unsafe_allocations: bool,
}

impl AstInspectionVisitor {
    fn record_domain_keyword(&mut self, text: &str) {
        let lower = text.to_lowercase();

        // 1. Domain::Compute
        let compute_keywords = [
            "tensor",
            "simd",
            "matmul",
            "activation",
            "relu",
            "gelu",
            "linear_algebra",
            "quantiz",
            "quant",
            "gemm",
            "matrix",
            "vector",
            "dot_product",
            "f32x8",
            "f64x4",
            "gradient",
            "forward_pass",
            "backward_pass",
            "convolution",
            "conv2d",
            "dimension",
            "shape",
            "strides",
            "candle",
            "burn",
            "ndarray",
            "raw_pointer_math",
        ];
        for kw in &compute_keywords {
            if lower.contains(kw) {
                *self.domain_hits.entry("compute").or_insert(0) += 1;
                self.total_tokens_matched += 1;
            }
        }

        // 2. Domain::Hypervisor
        let hypervisor_keywords = [
            "hypervisor",
            "supervisory",
            "lifecycle",
            "thread_affinity",
            "cpu_scheduling",
            "os_boundary",
            "sandbox",
            "governor",
            "metabolic",
            "tick_rate",
            "duty_cycle",
            "heartbeat",
            "fault_injector",
            "watchdog",
            "kernel",
            "core_affinity",
            "subsystem_health",
        ];
        for kw in &hypervisor_keywords {
            if lower.contains(kw) {
                *self.domain_hits.entry("hypervisor").or_insert(0) += 1;
                self.total_tokens_matched += 1;
            }
        }

        // 3. Domain::IpcBus
        let ipc_bus_keywords = [
            "ipc",
            "shm",
            "shared_memory",
            "ring_buffer",
            "swmr",
            "lock_free",
            "atomic_sequence",
            "channel",
            "mmap",
            "pod",
            "zeroable",
            "memory_mapped",
            "cross_process",
            "producer_consumer",
            "slot",
            "packet_ring",
            "header",
            "bytemuck",
            "payload_offset",
        ];
        for kw in &ipc_bus_keywords {
            if lower.contains(kw) {
                *self.domain_hits.entry("ipc_bus").or_insert(0) += 1;
                self.total_tokens_matched += 1;
            }
        }

        // 4. Domain::Orchestrator
        let orchestrator_keywords = [
            "agent",
            "workflow",
            "execution_plan",
            "task_dag",
            "state_machine",
            "coordinator",
            "decision_engine",
            "plan_step",
            "task_queue",
            "dispatcher",
            "step_graph",
            "dependency_graph",
            "transition",
            "action_executor",
            "swarm",
            "delegat",
        ];
        for kw in &orchestrator_keywords {
            if lower.contains(kw) {
                *self.domain_hits.entry("orchestrator").or_insert(0) += 1;
                self.total_tokens_matched += 1;
            }
        }
    }

    fn check_ambient_path(&mut self, path: &SynPath, line: usize, column: usize) {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let full = segments.join("::");

        // Check std::env calls and imports
        if full == "std::env" || full.starts_with("std::env::") || full.starts_with("env::") {
            self.ambient_risks.push(AmbientRiskSite {
                line,
                column,
                symbol: full.clone(),
                remediation: "Direct `std::env` access violates Zero-Ambient-Authority. Inject configuration structs."
                    .to_string(),
            });
        }

        // Check std::fs direct reads outside configuration handles
        if full == "std::fs" || full.starts_with("std::fs::") {
            self.ambient_risks.push(AmbientRiskSite {
                line,
                column,
                symbol: full,
                remediation: "Direct `std::fs` calls violate sandbox isolation. Inject explicit workspace storage handles."
                    .to_string(),
            });
        }
    }

    fn inspect_use_tree(&mut self, tree: &UseTree, prefix: &str) {
        match tree {
            UseTree::Path(UsePath { ident, tree, .. }) => {
                let current = if prefix.is_empty() {
                    ident.to_string()
                } else {
                    format!("{prefix}::{ident}")
                };
                if prefix.is_empty() {
                    let root_ident = ident.to_string();
                    if root_ident != "crate" && root_ident != "super" && root_ident != "self" {
                        self.detected_dependencies.insert(root_ident);
                    }
                }
                self.inspect_use_tree(tree, &current);
            }
            UseTree::Name(name) => {
                let full = if prefix.is_empty() {
                    name.ident.to_string()
                } else {
                    format!("{prefix}::{}", name.ident)
                };
                if prefix.is_empty() {
                    let root_ident = name.ident.to_string();
                    if root_ident != "crate" && root_ident != "super" && root_ident != "self" {
                        self.detected_dependencies.insert(root_ident);
                    }
                }
                self.record_domain_keyword(&full);
                if full.starts_with("std::env") || full.starts_with("std::fs") {
                    let span = name.span().start();
                    self.ambient_risks.push(AmbientRiskSite {
                        line: span.line,
                        column: span.column + 1,
                        symbol: full,
                        remediation: "Prohibited ambient system import. Pass paths and configs explicitly via typed structs."
                            .to_string(),
                    });
                }
            }
            UseTree::Rename(UseRename { ident, rename, .. }) => {
                let full = if prefix.is_empty() {
                    ident.to_string()
                } else {
                    format!("{prefix}::{ident}")
                };
                self.record_domain_keyword(&full);
                self.record_domain_keyword(&rename.to_string());
                if full.starts_with("std::env") || full.starts_with("std::fs") {
                    let span = ident.span().start();
                    self.ambient_risks.push(AmbientRiskSite {
                        line: span.line,
                        column: span.column + 1,
                        symbol: full,
                        remediation: "Prohibited ambient system import. Pass paths and configs explicitly via typed structs."
                            .to_string(),
                    });
                }
            }
            UseTree::Glob(glob) => {
                let full = format!("{prefix}::*");
                self.record_domain_keyword(&full);
                if prefix.starts_with("std::env") || prefix.starts_with("std::fs") {
                    let span = glob.span().start();
                    self.ambient_risks.push(AmbientRiskSite {
                        line: span.line,
                        column: span.column + 1,
                        symbol: full,
                        remediation: "Prohibited wildcard ambient system import. Pass configs explicitly via typed structs."
                            .to_string(),
                    });
                }
            }
            UseTree::Group(UseGroup { items, .. }) => {
                for item in items {
                    self.inspect_use_tree(item, prefix);
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for AstInspectionVisitor {
    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        self.inspect_use_tree(&node.tree, "");
        visit::visit_item_use(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        let name = node.ident.to_string();
        self.record_domain_keyword(&name);

        for field in &node.fields {
            if let Some(ident) = &field.ident {
                self.record_domain_keyword(&ident.to_string());
            }
        }

        if matches!(node.vis, Visibility::Public(_)) {
            let start = node.ident.span().start();
            self.public_items.push(PublicItem {
                kind: ItemKind::Struct,
                name: name.clone(),
                signature: format!("pub struct {}", name),
                line: start.line,
            });
        }

        visit::visit_item_struct(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        let name = node.ident.to_string();
        self.record_domain_keyword(&name);

        if matches!(node.vis, Visibility::Public(_)) {
            let start = node.ident.span().start();
            self.public_items.push(PublicItem {
                kind: ItemKind::Trait,
                name: name.clone(),
                signature: format!("pub trait {}", name),
                line: start.line,
            });
        }

        visit::visit_item_trait(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast ItemEnum) {
        let name = node.ident.to_string();
        self.record_domain_keyword(&name);

        for variant in &node.variants {
            self.record_domain_keyword(&variant.ident.to_string());
        }

        if matches!(node.vis, Visibility::Public(_)) {
            let start = node.ident.span().start();
            self.public_items.push(PublicItem {
                kind: ItemKind::Enum,
                name: name.clone(),
                signature: format!("pub enum {}", name),
                line: start.line,
            });
        }

        visit::visit_item_enum(self, node);
    }

    fn visit_item_type(&mut self, node: &'ast ItemType) {
        let name = node.ident.to_string();
        self.record_domain_keyword(&name);

        if matches!(node.vis, Visibility::Public(_)) {
            let start = node.ident.span().start();
            self.public_items.push(PublicItem {
                kind: ItemKind::TypeAlias,
                name: name.clone(),
                signature: format!("pub type {}", name),
                line: start.line,
            });
        }

        visit::visit_item_type(self, node);
    }

    fn visit_item_const(&mut self, node: &'ast ItemConst) {
        let name = node.ident.to_string();
        self.record_domain_keyword(&name);

        if matches!(node.vis, Visibility::Public(_)) {
            let start = node.ident.span().start();
            self.public_items.push(PublicItem {
                kind: ItemKind::Constant,
                name: name.clone(),
                signature: format!("pub const {}", name),
                line: start.line,
            });
        }

        visit::visit_item_const(self, node);
    }

    fn visit_item_static(&mut self, node: &'ast ItemStatic) {
        let name = node.ident.to_string();
        self.record_domain_keyword(&name);

        if matches!(node.vis, Visibility::Public(_)) {
            let start = node.ident.span().start();
            self.public_items.push(PublicItem {
                kind: ItemKind::Static,
                name: name.clone(),
                signature: format!("pub static {}", name),
                line: start.line,
            });
        }

        visit::visit_item_static(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let name = node.sig.ident.to_string();
        self.record_domain_keyword(&name);

        if matches!(node.vis, Visibility::Public(_)) {
            let start = node.sig.ident.span().start();

            let mut params = Vec::new();
            for input in &node.sig.inputs {
                match input {
                    FnArg::Receiver(r) => {
                        if matches!(r.kind, ReceiverKind::Reference(..)) {
                            if r.mutability.is_some() {
                                params.push("&mut self".to_string());
                            } else {
                                params.push("&self".to_string());
                            }
                        } else {
                            params.push("self".to_string());
                        }
                    }
                    FnArg::Typed(pat_type) => {
                        let pat_str = syn_pat_to_string(&pat_type.pat);
                        let ty_str = syn_type_to_string(&pat_type.ty);
                        params.push(format!("{}: {}", pat_str, ty_str));
                    }
                }
            }

            let ret_str = match &node.sig.output {
                ReturnType::Default => "".to_string(),
                ReturnType::Type(_, ty) => format!(" -> {}", syn_type_to_string(ty)),
            };

            let async_prefix = if node.sig.asyncness.is_some() {
                "async "
            } else {
                ""
            };
            let signature = format!(
                "pub {}fn {}({}){}",
                async_prefix,
                name,
                params.join(", "),
                ret_str
            );

            self.public_items.push(PublicItem {
                kind: ItemKind::Function,
                name: name.clone(),
                signature,
                line: start.line,
            });
        }

        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        let name = node.sig.ident.to_string();
        self.record_domain_keyword(&name);

        if matches!(node.vis, Visibility::Public(_)) {
            let start = node.sig.ident.span().start();

            let mut params = Vec::new();
            for input in &node.sig.inputs {
                match input {
                    FnArg::Receiver(r) => {
                        if matches!(r.kind, ReceiverKind::Reference(..)) {
                            if r.mutability.is_some() {
                                params.push("&mut self".to_string());
                            } else {
                                params.push("&self".to_string());
                            }
                        } else {
                            params.push("self".to_string());
                        }
                    }
                    FnArg::Typed(pat_type) => {
                        let pat_str = syn_pat_to_string(&pat_type.pat);
                        let ty_str = syn_type_to_string(&pat_type.ty);
                        params.push(format!("{}: {}", pat_str, ty_str));
                    }
                }
            }

            let ret_str = match &node.sig.output {
                ReturnType::Default => "".to_string(),
                ReturnType::Type(_, ty) => format!(" -> {}", syn_type_to_string(ty)),
            };

            let async_prefix = if node.sig.asyncness.is_some() {
                "async "
            } else {
                ""
            };
            let signature = format!(
                "pub {}fn {}({}){}",
                async_prefix,
                name,
                params.join(", "),
                ret_str
            );

            self.public_items.push(PublicItem {
                kind: ItemKind::Function,
                name: name.clone(),
                signature,
                line: start.line,
            });
        }

        visit::visit_impl_item_fn(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let syn::Expr::Path(expr_path) = &*node.func {
            let start = expr_path.span().start();
            self.check_ambient_path(&expr_path.path, start.line, start.column + 1);

            let last_ident = expr_path.path.segments.last().map(|s| s.ident.to_string());
            if let Some(ident) = last_ident {
                self.record_domain_keyword(&ident);
                if ident == "panic" {
                    self.raw_panic_count += 1;
                }
            }
        }

        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method = node.method.to_string();
        self.record_domain_keyword(&method);

        if method == "unwrap" || method == "expect" {
            self.raw_panic_count += 1;
        }

        visit::visit_expr_method_call(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let path_str = node
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        if path_str == "panic" || path_str == "todo" || path_str == "unimplemented" {
            self.raw_panic_count += 1;
        }

        self.record_domain_keyword(&path_str);
        visit::visit_macro(self, node);
    }
}

fn syn_pat_to_string(pat: &syn::Pat) -> String {
    match pat {
        syn::Pat::Ident(p) => p.ident.to_string(),
        _ => "_".to_string(),
    }
}

fn syn_type_to_string(ty: &Type) -> String {
    match ty {
        Type::Path(p) => p
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        Type::Reference(r) => {
            let mut_str = if r.mutability.is_some() { "mut " } else { "" };
            format!("&{}{}", mut_str, syn_type_to_string(&r.elem))
        }
        Type::Slice(s) => format!("[{}]", syn_type_to_string(&s.elem)),
        Type::Array(a) => format!("[{}; N]", syn_type_to_string(&a.elem)),
        Type::Tuple(t) => {
            let elems: Vec<String> = t.elems.iter().map(syn_type_to_string).collect();
            format!("({})", elems.join(", "))
        }
        _ => "T".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_domain_classification() {
        let classifier = DomainClassifier::default();
        let source = r#"
            use candle::Tensor;
            use ndarray::Array2;

            pub struct GemmKernel {
                pub matrix_a: Tensor,
                pub matrix_b: Tensor,
            }

            impl GemmKernel {
                pub fn execute_simd_matmul(&self) -> Tensor {
                    let dot_product = self.matrix_a.matmul(&self.matrix_b).unwrap();
                    let activation = dot_product.relu();
                    activation
                }
            }
        "#;

        let report = classifier
            .classify_source("gemm_kernel.rs", source)
            .unwrap();
        assert_eq!(report.target_domain, Domain::Compute);
        assert!(report.affinity_scores["compute"] > 0.60);
        assert_eq!(report.public_items.len(), 2);
        assert!(report.detected_dependencies.contains("candle"));
        assert!(report.detected_dependencies.contains("ndarray"));
    }

    #[test]
    fn test_hypervisor_domain_classification() {
        let classifier = DomainClassifier::default();
        let source = r#"
            pub struct SupervisoryController {
                pub tick_rate_hz: u32,
                pub thread_affinity: usize,
            }

            impl SupervisoryController {
                pub fn check_subsystem_health(&self) -> bool {
                    let watchdog_active = true;
                    let duty_cycle_governor = 1.0;
                    watchdog_active
                }
            }
        "#;

        let report = classifier
            .classify_source("supervisory.rs", source)
            .unwrap();
        assert_eq!(report.target_domain, Domain::Hypervisor);
        assert!(report.affinity_scores["hypervisor"] > 0.60);
        assert_eq!(report.public_items.len(), 2);
    }

    #[test]
    fn test_ipc_bus_domain_classification() {
        let classifier = DomainClassifier::default();
        let source = r#"
            use bytemuck::{Pod, Zeroable};

            #[repr(C)]
            #[derive(Clone, Copy)]
            pub struct ShmRingBuffer {
                pub slot_offset: u64,
                pub atomic_sequence: u64,
            }

            pub fn poll_swmr_packet_ring(buffer: &ShmRingBuffer) -> u64 {
                buffer.atomic_sequence
            }
        "#;

        let report = classifier.classify_source("shm_ring.rs", source).unwrap();
        assert_eq!(report.target_domain, Domain::IpcBus);
        assert!(report.affinity_scores["ipc_bus"] > 0.60);
        assert_eq!(report.public_items.len(), 2);
        assert!(report.detected_dependencies.contains("bytemuck"));
    }

    #[test]
    fn test_orchestrator_domain_classification() {
        let classifier = DomainClassifier::default();
        let source = r#"
            pub struct AgentExecutionPlan {
                pub task_dag: Vec<String>,
                pub step_graph: Vec<usize>,
            }

            pub trait WorkflowCoordinator {
                fn transition_state_machine(&mut self, next: String);
                fn execute_decision_engine_plan(&self);
            }
        "#;

        let report = classifier.classify_source("workflow.rs", source).unwrap();
        assert_eq!(report.target_domain, Domain::Orchestrator);
        assert!(report.affinity_scores["orchestrator"] > 0.60);
        assert_eq!(report.public_items.len(), 2);
    }

    #[test]
    fn test_novel_domain_below_threshold() {
        let classifier = DomainClassifier::default();
        let source = r#"
            pub struct CustomWidget {
                pub red: u8,
                pub green: u8,
                pub blue: u8,
            }

            pub fn paint_button() {}
        "#;

        let report = classifier.classify_source("widget.rs", source).unwrap();
        match report.target_domain {
            Domain::Novel(name) => {
                assert!(name.contains("widget") || name.contains("unclassified"));
            }
            other => panic!("Expected Domain::Novel, got {:?}", other),
        }
    }

    #[test]
    fn test_ambient_authority_and_risk_detection() {
        let classifier = DomainClassifier::default();
        let source = r#"
            use std::env;
            use std::fs;

            pub fn load_credentials() {
                let token = std::env::var("SECRET_TOKEN").unwrap();
                let file_data = std::fs::read_to_string("/tmp/config.json").expect("read failed");
                println!("{}", token);
            }
        "#;

        let report = classifier
            .classify_source("credentials.rs", source)
            .unwrap();
        assert!(
            !report.ambient_risks.is_empty(),
            "Must detect ambient risks"
        );
        assert!(
            report
                .ambient_risks
                .iter()
                .any(|r| r.symbol.contains("env")),
            "Must flag std::env access"
        );
        assert!(
            report.ambient_risks.iter().any(|r| r.symbol.contains("fs")),
            "Must flag std::fs access"
        );
        assert!(!report.required_transformations.is_empty());
        assert!(
            report
                .required_transformations
                .iter()
                .any(|t| t.contains("Zero-Ambient-Authority"))
        );
    }
}
