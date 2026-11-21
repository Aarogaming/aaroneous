//! Vocabulary rule: analogical (biology-derived) names where an engineering
//! term exists.
//!
//! Owner rule (2026-11-19): use a standardized scientific or engineering term
//! if one exists for the intent; the analogical name is allowed only when no
//! such term exists. The owner approved the term list on 2026-11-21
//! (`docs/BIOLOGY_TERM_LIST_PROPOSAL_2026-11-20.md`: list A flag, list B
//! allow, `synapse` flagged).
//!
//! This module is pure: it parses a term list from text and counts flagged
//! identifier occurrences in one parsed source file. Reading the term list,
//! walking the workspace and comparing against the per-package baseline are the
//! caller's job (`cargo xtask check-vocabulary`).
//!
//! Matching is on identifier *words*, not substrings. An identifier is split on
//! `_` and on camelCase boundaries (`SharedMemorySynapse` is `shared`,
//! `memory`, `synapse`; `ModelDNA` is `model`, `dna`; `SI_FLAG_TIER_1_CORTEX`
//! is `si`, `flag`, `tier`, `1`, `cortex`). Only identifiers are inspected, not
//! comments or strings. Identifiers inside macro invocations are not visited
//! (syn keeps macro bodies as raw tokens), so this is a lower bound.

use proc_macro2::Span;
use std::path::{Path, PathBuf};
use syn::visit::{self, Visit};
use syn::{Attribute, Ident, ImplItemFn, ItemFn, ItemImpl, ItemMod, TraitItemFn};

use super::unwrap_panic::is_test_item;

/// How a term is compared with an identifier word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    /// The word starts with the stem (`synap` matches `synapse`, `synaptic`).
    Prefix,
    /// The word equals the stem (`dna`, `hox`).
    Exact,
}

impl MatchKind {
    fn matches(self, word: &str, stem: &str) -> bool {
        match self {
            Self::Prefix => word.starts_with(stem),
            Self::Exact => word == stem,
        }
    }
}

/// One listed term and the reason it is listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    pub kind: MatchKind,
    pub stem: String,
    pub reason: String,
}

/// An allowed phrase: a flagged word is not counted when it starts these
/// adjacent words (`genetic algorithm`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowedPhrase {
    pub words: Vec<String>,
    pub reason: String,
}

/// The reviewed term list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TermList {
    pub flag: Vec<Term>,
    pub allow: Vec<Term>,
    pub allow_phrases: Vec<AllowedPhrase>,
}

impl TermList {
    /// Parse the term-list text.
    ///
    /// One entry per line, fields separated by `|`, `#` starts a comment line:
    ///
    /// ```text
    /// flag|prefix|synap|reason
    /// flag|exact|dna|reason
    /// allow|prefix|mutat|reason
    /// allow-phrase|genetic algorithm|reason
    /// ```
    ///
    /// Every entry needs a non-empty reason, so an allowed term is recorded as
    /// allowed rather than left to a reviewer's memory. Malformed lines are an
    /// error (fail closed).
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut list = Self::default();
        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let lineno = idx + 1;
            let fields: Vec<&str> = line.split('|').map(str::trim).collect();
            let reason_of = |field: Option<&&str>| -> Result<String, String> {
                match field {
                    Some(r) if !r.is_empty() => Ok((*r).to_string()),
                    _ => Err(format!("line {lineno}: every entry needs a reason")),
                }
            };
            match fields.first().copied() {
                Some(tag @ ("flag" | "allow")) => {
                    let kind = match fields.get(1).copied() {
                        Some("prefix") => MatchKind::Prefix,
                        Some("exact") => MatchKind::Exact,
                        other => {
                            return Err(format!(
                                "line {lineno}: expected `prefix` or `exact`, found {other:?}"
                            ));
                        }
                    };
                    let stem = match fields.get(2) {
                        Some(s) if !s.is_empty() => s.to_lowercase(),
                        _ => return Err(format!("line {lineno}: missing term")),
                    };
                    if !stem.chars().all(|c| c.is_ascii_alphanumeric()) {
                        return Err(format!(
                            "line {lineno}: a term is one lowercase word, found `{stem}`"
                        ));
                    }
                    let reason = reason_of(fields.get(3))?;
                    let term = Term { kind, stem, reason };
                    if tag == "flag" {
                        list.flag.push(term);
                    } else {
                        list.allow.push(term);
                    }
                }
                Some("allow-phrase") => {
                    let words: Vec<String> = fields
                        .get(1)
                        .map(|p| p.split_whitespace().map(str::to_lowercase).collect())
                        .unwrap_or_default();
                    if words.len() < 2 {
                        return Err(format!(
                            "line {lineno}: an allowed phrase needs at least two words"
                        ));
                    }
                    let reason = reason_of(fields.get(2))?;
                    list.allow_phrases.push(AllowedPhrase { words, reason });
                }
                other => {
                    return Err(format!(
                        "line {lineno}: unknown entry type {other:?} (expected flag, allow or allow-phrase)"
                    ));
                }
            }
        }
        Ok(list)
    }

    /// Flagged terms found in `ident`, one entry per flagged word, as the
    /// matching list stem. Empty when the identifier is clean.
    pub fn flagged_stems(&self, ident: &str) -> Vec<String> {
        let words = split_words(ident);
        let mut found = Vec::new();
        for (i, word) in words.iter().enumerate() {
            if self
                .allow
                .iter()
                .any(|t| t.kind.matches(word, t.stem.as_str()))
            {
                continue;
            }
            let Some(term) = self
                .flag
                .iter()
                .find(|t| t.kind.matches(word, t.stem.as_str()))
            else {
                continue;
            };
            let in_allowed_phrase = self.allow_phrases.iter().any(|p| {
                words.len() >= i + p.words.len() && words[i..i + p.words.len()] == p.words[..]
            });
            if in_allowed_phrase {
                continue;
            }
            found.push(term.stem.clone());
        }
        found
    }
}

/// Split an identifier into lowercase words on `_`, digits and camelCase
/// boundaries. A run of capitals is one word until the last capital that starts
/// a lowercase run (`HTTPServer` is `http`, `server`).
pub fn split_words(ident: &str) -> Vec<String> {
    let ident = ident.strip_prefix("r#").unwrap_or(ident);
    let mut words = Vec::new();
    for segment in ident.split(|c: char| !c.is_ascii_alphanumeric()) {
        if segment.is_empty() {
            continue;
        }
        let chars: Vec<char> = segment.chars().collect();
        let mut start = 0;
        for i in 1..chars.len() {
            let prev = chars[i - 1];
            let cur = chars[i];
            let next = chars.get(i + 1).copied();
            let boundary = (prev.is_ascii_lowercase() && cur.is_ascii_uppercase())
                || (prev.is_ascii_alphabetic() && cur.is_ascii_digit())
                || (prev.is_ascii_digit() && cur.is_ascii_alphabetic())
                || (prev.is_ascii_uppercase()
                    && cur.is_ascii_uppercase()
                    && next.is_some_and(|n| n.is_ascii_lowercase()));
            if boundary {
                words.push(chars[start..i].iter().collect::<String>().to_lowercase());
                start = i;
            }
        }
        words.push(chars[start..].iter().collect::<String>().to_lowercase());
    }
    words
}

/// One flagged identifier occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyHit {
    pub file_path: PathBuf,
    pub line: usize,
    pub ident: String,
    pub term: String,
}

/// Counts flagged identifiers outside `#[cfg(test)]` / `#[test]` items.
pub struct VocabularyVisitor<'a> {
    file_path: &'a Path,
    terms: &'a TermList,
    pub hits: Vec<VocabularyHit>,
    test_depth: usize,
}

impl<'a> VocabularyVisitor<'a> {
    pub fn new(file_path: &'a Path, terms: &'a TermList) -> Self {
        Self {
            file_path,
            terms,
            hits: Vec::new(),
            test_depth: 0,
        }
    }

    fn enter_if_test<F: FnOnce(&mut Self)>(&mut self, attrs: &[Attribute], visit_inner: F) {
        let is_test = is_test_item(attrs);
        if is_test {
            self.test_depth += 1;
        }
        visit_inner(self);
        if is_test {
            self.test_depth -= 1;
        }
    }

    fn record(&mut self, ident: &Ident, span: Span) {
        if self.test_depth > 0 {
            return;
        }
        let name = ident.to_string();
        for term in self.terms.flagged_stems(&name) {
            self.hits.push(VocabularyHit {
                file_path: self.file_path.to_path_buf(),
                line: span.start().line,
                ident: name.clone(),
                term,
            });
        }
    }
}

impl<'ast> Visit<'ast> for VocabularyVisitor<'_> {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        let attrs = node.attrs.clone();
        self.enter_if_test(&attrs, |this| visit::visit_item_mod(this, node));
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let attrs = node.attrs.clone();
        self.enter_if_test(&attrs, |this| visit::visit_item_fn(this, node));
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let attrs = node.attrs.clone();
        self.enter_if_test(&attrs, |this| visit::visit_item_impl(this, node));
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        let attrs = node.attrs.clone();
        self.enter_if_test(&attrs, |this| visit::visit_impl_item_fn(this, node));
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        let attrs = node.attrs.clone();
        self.enter_if_test(&attrs, |this| visit::visit_trait_item_fn(this, node));
    }

    fn visit_ident(&mut self, node: &'ast Ident) {
        self.record(node, node.span());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "\
# test list
flag|prefix|synap|channel or bus is the standard term
flag|prefix|symbio|cooperative
flag|exact|dna|manifest
flag|exact|hox|registry
flag|prefix|genetic|parameter set
flag|prefix|cortex|tier
flag|prefix|metabol|resource budget
allow|prefix|mutat|code mutation is a standard term
allow-phrase|genetic algorithm|standard optimization term
";

    fn list() -> TermList {
        TermList::parse(LIST).expect("test list parses")
    }

    fn hits(source: &str) -> Vec<(String, String)> {
        let terms = list();
        let ast: syn::File = syn::parse_str(source).expect("test source parses");
        let mut visitor = VocabularyVisitor::new(Path::new("t.rs"), &terms);
        visitor.visit_file(&ast);
        visitor
            .hits
            .into_iter()
            .map(|h| (h.ident, h.term))
            .collect()
    }

    #[test]
    fn splits_snake_camel_acronym_and_digits() {
        assert_eq!(
            split_words("SharedMemorySynapse"),
            ["shared", "memory", "synapse"]
        );
        assert_eq!(split_words("ModelDNA"), ["model", "dna"]);
        assert_eq!(
            split_words("SI_FLAG_TIER_1_CORTEX"),
            ["si", "flag", "tier", "1", "cortex"]
        );
        assert_eq!(split_words("HTTPServer"), ["http", "server"]);
        assert_eq!(split_words("r#type"), ["type"]);
    }

    #[test]
    fn flags_listed_words_in_every_identifier_position() {
        let found = hits(
            "struct SynapseState { dna_hash: u8 }\nfn load_dna_sidecar() { let hox = 1; }\nenum Mode { Symbiotic }\n",
        );
        let names: Vec<&str> = found.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                "SynapseState",
                "dna_hash",
                "load_dna_sidecar",
                "hox",
                "Symbiotic"
            ]
        );
    }

    #[test]
    fn matches_whole_words_not_substrings() {
        // `dna` inside another word, `hox` inside `shoxx`, `synap` mid-word.
        assert!(hits("fn redna() {} fn shoxx() {} fn asynapse() {}").is_empty());
        // `dna` / `hox` are exact words, so a longer word does not match.
        assert!(hits("fn dnaish() {} fn hoxes() {}").is_empty());
    }

    #[test]
    fn allow_list_and_phrases_are_not_flagged() {
        assert!(hits("fn code_mutator() {} struct MutationIntent;").is_empty());
        assert!(hits("fn run_genetic_algorithm() {}").is_empty());
        // The phrase exempts only that pairing; plain `genetic` is still flagged.
        let found = hits("fn genetic_loci() {} fn genetic_algorithm_step() {}");
        assert_eq!(
            found,
            vec![("genetic_loci".to_string(), "genetic".to_string())]
        );
    }

    #[test]
    fn one_hit_per_flagged_word() {
        let found = hits("fn synapse_metabolism() {}");
        assert_eq!(
            found,
            vec![
                ("synapse_metabolism".to_string(), "synap".to_string()),
                ("synapse_metabolism".to_string(), "metabol".to_string()),
            ]
        );
    }

    #[test]
    fn skips_cfg_test_and_test_items() {
        let src = "#[cfg(test)]\nmod tests { fn synapse_case() {} }\n#[test]\nfn dna_check() {}\nfn kept_synapse() {}\n";
        let found = hits(src);
        assert_eq!(
            found,
            vec![("kept_synapse".to_string(), "synap".to_string())]
        );
    }

    #[test]
    fn comments_and_strings_are_not_inspected() {
        assert!(hits("// synapse dna\nfn f() { let _ = \"synapse\"; }").is_empty());
    }

    #[test]
    fn records_line_numbers() {
        let terms = list();
        let src = "fn ok() {}\n\nfn synapse() {}\n";
        let ast: syn::File = syn::parse_str(src).expect("parses");
        let mut v = VocabularyVisitor::new(Path::new("t.rs"), &terms);
        v.visit_file(&ast);
        assert_eq!(v.hits.len(), 1);
        assert_eq!(v.hits[0].line, 3);
    }

    #[test]
    fn parse_rejects_malformed_lines() {
        assert!(
            TermList::parse("flag|prefix|synap").is_err(),
            "reason is required"
        );
        assert!(TermList::parse("flag|suffix|synap|r").is_err());
        assert!(TermList::parse("flag|prefix||r").is_err());
        assert!(TermList::parse("flag|prefix|two words|r").is_err());
        assert!(TermList::parse("allow-phrase|single|r").is_err());
        assert!(TermList::parse("ban|prefix|x|r").is_err());
        assert!(
            TermList::parse("# only a comment\n\n")
                .expect("ok")
                .flag
                .is_empty()
        );
    }
}
