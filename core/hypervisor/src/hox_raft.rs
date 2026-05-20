use openraft::Config;
use openraft::Raft;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use openraft::storage::LogState;
use openraft::RaftLogReader;
use openraft::RaftStorage;
use openraft::RaftNetwork;
use openraft::RaftNetworkFactory;
use openraft::storage::RaftSnapshotBuilder;
use openraft::Vote;
use openraft::LogId;
use openraft::Snapshot;
use std::collections::HashMap;
use tokio::sync::RwLock;
use rocksdb::{DB, Options, ColumnFamilyDescriptor};
use std::path::Path;
use crate::nats_client::{NatsClient, NatsClientConfig};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HoxRequest {
    pub op: HoxOp,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HoxOp {
    Register(String, String),
    Revoke(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HoxResponse {
    pub success: bool,
}

pub type NodeId = u64;

openraft::declare_raft_types!(
    pub HoxTypeConfig:
        D = HoxRequest,
        R = HoxResponse,
        NodeId = NodeId,
        Node = (),
        Entry = openraft::entry::Entry<HoxTypeConfig>,
        SnapshotData = Vec<u8>
);

pub struct HoxNetwork {
    nats: Arc<NatsClient>,
}

impl HoxNetwork {
    pub fn new(nats: Arc<NatsClient>) -> Self {
        Self { nats }
    }
}

#[async_trait]
impl RaftNetworkFactory<HoxTypeConfig> for HoxNetwork {
    type Network = HoxNetworkConnection;
    async fn new_client(&mut self, target: NodeId, _node: &()) -> Self::Network {
        HoxNetworkConnection {
            nats: self.nats.clone(),
            target,
        }
    }
}

pub struct HoxNetworkConnection {
    nats: Arc<NatsClient>,
    target: NodeId,
}

#[async_trait]
impl RaftNetwork<HoxTypeConfig> for HoxNetworkConnection {
    async fn send_append_entries(&mut self, rpc: openraft::rpc::AppendEntriesRequest<HoxTypeConfig>) -> Result<openraft::rpc::AppendEntriesResponse<NodeId>, openraft::error::RPCError<NodeId, openraft::error::RaftError<NodeId>>> {
        let subject = format!("hox.raft.append.{}", self.target);
        let payload = serde_json::to_vec(&rpc).map_err(|e| openraft::error::RPCError::Network(openraft::error::NetworkError::new(e.into())))?;
        
        // This is a simplification; real NATS request-reply would be better.
        // For now, we use a mock-style implementation or assume NATS request-reply exists.
        Err(openraft::error::RPCError::Network(openraft::error::NetworkError::new(anyhow!("NATS RPC not fully wired")).into()))
    }
    async fn send_install_snapshot(&mut self, _rpc: openraft::rpc::InstallSnapshotRequest<HoxTypeConfig>) -> Result<openraft::rpc::InstallSnapshotResponse<NodeId>, openraft::error::RPCError<NodeId, openraft::error::RaftError<NodeId>>> {
        Err(openraft::error::RPCError::Network(openraft::error::NetworkError::new(anyhow!("Network not implemented")).into()))
    }
    async fn send_vote(&mut self, rpc: openraft::rpc::VoteRequest<NodeId>) -> Result<openraft::rpc::VoteResponse<NodeId>, openraft::error::RPCError<NodeId, openraft::error::RaftError<NodeId>>> {
        let subject = format!("hox.raft.vote.{}", self.target);
        let payload = serde_json::to_vec(&rpc).map_err(|e| openraft::error::RPCError::Network(openraft::error::NetworkError::new(e.into())))?;
        Err(openraft::error::RPCError::Network(openraft::error::NetworkError::new(anyhow!("NATS RPC not fully wired")).into()))
    }
}

pub struct HoxStorage {
    db: Arc<DB>,
    /// The log data.
    log: RwLock<HashMap<u64, openraft::entry::Entry<HoxTypeConfig>>>,
    /// The state machine.
    sm: RwLock<HashMap<String, String>>,
    /// The last applied log id.
    last_applied_log_id: RwLock<Option<LogId<NodeId>>>,
    /// The current vote.
    vote: RwLock<Option<Vote<NodeId>>>,
}

impl HoxStorage {
    pub fn new(db_path: &Path) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_logs = ColumnFamilyDescriptor::new("logs", Options::default());
        let cf_sm = ColumnFamilyDescriptor::new("state_machine", Options::default());
        let cf_meta = ColumnFamilyDescriptor::new("meta", Options::default());

        let db = DB::open_cf_descriptors(&opts, db_path, vec![cf_logs, cf_sm, cf_meta])?;
        
        Ok(Self {
            db: Arc::new(db),
            log: RwLock::new(HashMap::new()),
            sm: RwLock::new(HashMap::new()),
            last_applied_log_id: RwLock::new(None),
            vote: RwLock::new(None),
        })
    }
}

#[async_trait]
impl RaftLogReader<HoxTypeConfig> for Arc<HoxStorage> {
    async fn get_log_state(&mut self) -> Result<LogState<HoxTypeConfig>, openraft::error::StorageError<NodeId>> {
        let log = self.log.read().await;
        let last = log.iter().max_by_key(|(&id, _)| id).map(|(_, ent)| *ent.get_log_id());
        let last_purged_log_id = None; // Simplified

        Ok(LogState {
            last_purged_log_id,
            last_log_id: last,
        })
    }

    async fn try_get_log_entries<RB: std::ops::RangeBounds<u64> + Send + std::fmt::Debug>(
        &mut self,
        range: RB,
    ) -> Result<Vec<openraft::entry::Entry<HoxTypeConfig>>, openraft::error::StorageError<NodeId>> {
        let log = self.log.read().await;
        let mut entries = Vec::new();
        for (_, entry) in log.iter() {
            if range.contains(&entry.log_id.index) {
                entries.push(entry.clone());
            }
        }
        entries.sort_by_key(|e| e.log_id.index);
        Ok(entries)
    }
}

#[async_trait]
impl RaftSnapshotBuilder<HoxTypeConfig> for Arc<HoxStorage> {
    async fn build_snapshot(&mut self) -> Result<Snapshot<HoxTypeConfig>, openraft::error::StorageError<NodeId>> {
        Err(openraft::error::StorageError::read_snapshot(None, anyhow!("Snapshot not implemented").into()))
    }
}

#[async_trait]
impl RaftStorage<HoxTypeConfig> for Arc<HoxStorage> {
    type LogReader = Self;
    type SnapshotBuilder = Self;

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }

    async fn save_vote(&mut self, vote: &Vote<NodeId>) -> Result<(), openraft::error::StorageError<NodeId>> {
        let mut v = self.vote.write().await;
        *v = Some(*vote);
        Ok(())
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<NodeId>>, openraft::error::StorageError<NodeId>> {
        Ok(*self.vote.read().await)
    }

    async fn append_to_log<I>(&mut self, entries: I) -> Result<(), openraft::error::StorageError<NodeId>>
    where
        I: IntoIterator<Item = openraft::entry::Entry<HoxTypeConfig>> + Send,
    {
        let mut log = self.log.write().await;
        for entry in entries {
            log.insert(entry.log_id.index, entry);
        }
        Ok(())
    }

    async fn delete_conflict_logs_since(
        &mut self,
        log_id: LogId<NodeId>,
    ) -> Result<(), openraft::error::StorageError<NodeId>> {
        let mut log = self.log.write().await;
        let keys: Vec<_> = log.keys().filter(|&&i| i >= log_id.index).cloned().collect();
        for key in keys {
            log.remove(&key);
        }
        Ok(())
    }

    async fn purge_logs_upto(&mut self, _log_id: LogId<NodeId>) -> Result<(), openraft::error::StorageError<NodeId>> {
        Ok(())
    }

    async fn last_applied_state(
        &mut self,
    ) -> Result<(Option<LogId<NodeId>>, openraft::StoredMembership<NodeId, ()>), openraft::error::StorageError<NodeId>> {
        let last_applied = self.last_applied_log_id.read().await;
        Ok((*last_applied, openraft::StoredMembership::default()))
    }

    async fn apply_to_state_machine(
        &mut self,
        entries: Vec<openraft::entry::Entry<HoxTypeConfig>>,
    ) -> Result<Vec<HoxResponse>, openraft::error::StorageError<NodeId>> {
        let mut sm = self.sm.write().await;
        let mut last_applied = self.last_applied_log_id.write().await;
        let mut res = Vec::new();

        for entry in entries {
            *last_applied = Some(entry.log_id);

            match entry.payload {
                openraft::EntryPayload::Normal(req) => {
                    match req.op {
                        HoxOp::Register(key, val) => {
                            sm.insert(key, val);
                            res.push(HoxResponse { success: true });
                        }
                        HoxOp::Revoke(key) => {
                            sm.remove(&key);
                            res.push(HoxResponse { success: true });
                        }
                    }
                }
                _ => {
                    res.push(HoxResponse { success: true });
                }
            }
        }

        Ok(res)
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<Vec<u8>>, openraft::error::StorageError<NodeId>> {
        Ok(Box::new(Vec::new()))
    }

    async fn install_snapshot(
        &mut self,
        _meta: &openraft::SnapshotMeta<NodeId, ()>,
        _snapshot: Box<Vec<u8>>,
    ) -> Result<(), openraft::error::StorageError<NodeId>> {
        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<Snapshot<HoxTypeConfig>>, openraft::error::StorageError<NodeId>> {
        Ok(None)
    }
}
pub struct HoxRaftNode {
    pub raft: Raft<HoxTypeConfig>,
}

impl HoxRaftNode {
    pub async fn new(node_id: NodeId, config: Arc<Config>, db_path: &Path, nats_url: &str) -> Result<Self> {
        println!("[HoxRaft] Initializing Raft node {} at {:?}", node_id, db_path);
        
        let storage = Arc::new(HoxStorage::new(db_path)?);
        
        let nats_config = NatsClientConfig {
            nats_url: nats_url.to_string(),
            ..Default::default()
        };
        let nats = Arc::new(NatsClient::new(nats_config));
        nats.connect().await.map_err(|e| anyhow!(e))?;
        
        let network = HoxNetwork::new(nats);
        
        let raft = Raft::new(node_id, config, network, storage).await?;
        
        Ok(Self { raft })
    }
}
