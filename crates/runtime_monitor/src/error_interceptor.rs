// src/error_interceptor.rs

//! Zero‑copy process error interceptor for the Runtime Monitor fast‑path.
//! Parses stderr, constructs a `Trigger` POD and pushes it onto a lock‑free queue.

use anyhow::Result;
use bytemuck::Pod;
use hypervisor::state::telemetry::Trigger;
use super::push_trigger;
