//! crates/mcp_server
//! Anthropic Model Context Protocol (2024-11-05 spec) HTTP/SSE and JSON-RPC
//! 2.0 server, wired directly to `capabilities::ToolRegistry` to expose
//! live tools to Claude Desktop, Cursor, and other MCP-compatible clients.
//!
//! Deliberately independent of `hypervisor`: everything this crate needs
//! from a running sovereign hive goes through `mcp_service::IntentBackend`,
//! which `hypervisor` implements for its own `Federation` type (see
//! `core/hypervisor/src/federation/cluster/mcp_backend.rs`) rather than
//! this crate depending on `hypervisor` directly — `hypervisor`'s own
//! binary depends on this crate, and Cargo forbids the reverse edge that
//! would create.

#![deny(unsafe_code)]

// Aliased so mcp_service's `crate::llm::` references (mirroring
// hypervisor's own `pub use llm_gateway as llm;`) resolve unchanged.
pub use llm_gateway as llm;

pub mod mcp_service;
