/// P2P Networking Module: Real Multi-Device Sync via Iroh
///
/// This module provides the bridge between Omnipresent specialist's abstract
/// device model and real peer-to-peer QUIC connections via the Iroh library.
///
/// # Feature Gating
///
/// The actual Iroh implementation is gated behind the `p2p-iroh` feature flag
/// because Iroh brings in heavy dependencies (QUIC stack, relay server client,
/// DNS resolution). Without the feature, a no-op stub implementation is used
/// so the rest of the system continues to work for testing and development.
///
/// # Usage
///
/// ```no_run
/// # async fn example() -> anyhow::Result<()> {
/// use a_run::federation::p2p::P2pNode;
///
/// let node = P2pNode::spawn(b"aaroneous/sync/v1").await?;
/// let endpoint_id = node.endpoint_id();
/// println!("Our P2P address: {}", endpoint_id);
/// # Ok(())
/// # }
/// ```
///
/// # Architecture
///
/// - `P2pNode`: Async-friendly handle around an Iroh `Endpoint`
/// - `P2pError`: Unified error type covering both stub and real implementations
/// - `SyncMessage`: Wire format for Intent sync between peers
///
/// When `p2p-iroh` is enabled, `P2pNode` wraps a real Iroh endpoint and provides
/// real QUIC connections. Otherwise, `P2pNode` is a stub that records calls but
/// does no actual networking.
pub mod types;

#[cfg(feature = "p2p-iroh")]
pub mod iroh_node;

#[cfg(not(feature = "p2p-iroh"))]
pub mod stub_node;

pub use types::{P2pError, P2pNodeId, SyncMessage};

#[cfg(feature = "p2p-iroh")]
pub use iroh_node::P2pNode;

#[cfg(not(feature = "p2p-iroh"))]
pub use stub_node::P2pNode;
