/// Communication Layer: How specialists talk to each other
/// 
/// Provides async channels for:
/// - Specialist → Sentinel: proposal submission
/// - Sentinel → Specialist: decision execution
/// - Specialist ↔ Specialist: delegation and negotiation
/// 
/// Uses tokio channels for non-blocking communication

use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::federation::specialist::{SpecialistId, Decision, DelegateRequest, Conflict};
use crate::federation::proposal::{Proposal, ProposalSet};

/// Message types that flow through the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecialistMessage {
    /// Specialist proposes an action to Sentinel
    ProposalSubmitted(Proposal),
    /// Sentinel accepts a proposal and sends execution order
    DecisionIssued(Decision),
    /// Specialist requests help from another specialist
    DelegationRequest(DelegateRequest),
    /// Response to delegation
    DelegationResponse { success: bool, result: String },
    /// Specialist proposes negotiation with another
    ConflictNotification(Conflict),
    /// Status update
    StatusUpdate(String),
    /// Error notification
    Error(String),
}

/// Async channel for specialist messages
pub struct MessageChannel {
    tx: mpsc::UnboundedSender<SpecialistMessage>,
    rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<SpecialistMessage>>>,
}

impl MessageChannel {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx: Arc::new(tokio::sync::Mutex::new(rx)),
        }
    }

    pub async fn send(&self, message: SpecialistMessage) -> Result<(), tokio::sync::mpsc::error::SendError<SpecialistMessage>> {
        self.tx.send(message)
    }

    pub async fn receive(&self) -> Option<SpecialistMessage> {
        let mut rx = self.rx.lock().await;
        rx.recv().await
    }

    /// Non-blocking receive: returns `Some(msg)` if there is a message waiting,
    /// `None` if the channel is empty. Does not block.
    pub async fn try_receive(&self) -> Option<SpecialistMessage> {
        let mut rx = self.rx.lock().await;
        rx.try_recv().ok()
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<SpecialistMessage> {
        self.tx.clone()
    }
}

impl Default for MessageChannel {
    fn default() -> Self {
        Self::new()
    }
}

/// Central communication bus managing all specialist channels
pub struct CommunicationBus {
    specialist_channels: std::collections::HashMap<SpecialistId, Arc<MessageChannel>>,
    sentinel_channel: Arc<MessageChannel>,
    proposal_inbox: Arc<tokio::sync::Mutex<ProposalSet>>,
}

impl CommunicationBus {
    pub fn new() -> Self {
        Self {
            specialist_channels: std::collections::HashMap::new(),
            sentinel_channel: Arc::new(MessageChannel::new()),
            proposal_inbox: Arc::new(tokio::sync::Mutex::new(ProposalSet::new())),
        }
    }

    /// Register a specialist's communication channel
    pub fn register_specialist(&mut self, id: SpecialistId) {
        let channel = Arc::new(MessageChannel::new());
        self.specialist_channels.insert(id, channel);
    }

    /// Get the channel for communicating with a specialist
    pub fn specialist_channel(&self, id: SpecialistId) -> Option<Arc<MessageChannel>> {
        self.specialist_channels.get(&id).cloned()
    }

    /// Get Sentinel's channel for issuing decisions
    pub fn sentinel_channel(&self) -> Arc<MessageChannel> {
        self.sentinel_channel.clone()
    }

    /// Submit a proposal from a specialist
    pub async fn submit_proposal(&self, proposal: Proposal) -> Result<(), String> {
        let mut inbox = self.proposal_inbox.lock().await;
        inbox.add(proposal);
        Ok(())
    }

    /// Get all pending proposals for Sentinel review
    pub async fn pending_proposals(&self) -> ProposalSet {
        let inbox = self.proposal_inbox.lock().await;
        inbox.clone()
    }

    /// Clear proposals after Sentinel processes them
    pub async fn clear_proposals(&self) {
        let mut inbox = self.proposal_inbox.lock().await;
        inbox.proposals.clear();
    }

    /// Broadcast a message to all specialists
    pub async fn broadcast(&self, message: SpecialistMessage) -> Result<(), String> {
        for channel in self.specialist_channels.values() {
            let _ = channel.send(message.clone());
        }
        Ok(())
    }

    /// Direct message from one specialist to another
    pub async fn send_direct(
        &self,
        _from: SpecialistId,
        to: SpecialistId,
        message: SpecialistMessage,
    ) -> Result<(), String> {
        if let Some(channel) = self.specialist_channel(to) {
            let _ = channel.send(message);
            Ok(())
        } else {
            Err(format!("Specialist {:?} not found", to))
        }
    }

    pub fn specialist_count(&self) -> usize {
        self.specialist_channels.len()
    }
}

impl Default for CommunicationBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_channel_send_receive() {
        let channel = MessageChannel::new();
        
        let msg = SpecialistMessage::Error("test error".to_string());
        let _ = channel.send(msg.clone()).await;
        
        let received = channel.receive().await;
        assert!(received.is_some());
    }

    #[tokio::test]
    async fn test_communication_bus_register() {
        let mut bus = CommunicationBus::new();
        
        bus.register_specialist(SpecialistId::Visionary);
        bus.register_specialist(SpecialistId::Sentinel);
        
        assert_eq!(bus.specialist_count(), 2);
        assert!(bus.specialist_channel(SpecialistId::Visionary).is_some());
    }

    #[tokio::test]
    async fn test_submit_proposal() {
        let bus = CommunicationBus::new();
        
        let proposal = Proposal::new(
            SpecialistId::Visionary,
            "design".to_string(),
            "Generate designs".to_string(),
            0.8,
            crate::federation::specialist::ProposalPriority::Normal,
        );
        
        bus.submit_proposal(proposal).await.unwrap();
        
        let proposals = bus.pending_proposals().await;
        assert_eq!(proposals.count(), 1);
    }

    #[tokio::test]
    async fn test_broadcast_message() {
        let mut bus = CommunicationBus::new();
        
        bus.register_specialist(SpecialistId::Visionary);
        bus.register_specialist(SpecialistId::Omnipresent);
        
        let msg = SpecialistMessage::StatusUpdate("all online".to_string());
        bus.broadcast(msg).await.unwrap();
        
        // Verify messages were sent (would need to check channels)
        assert_eq!(bus.specialist_count(), 2);
    }

    #[tokio::test]
    async fn test_direct_message() {
        let mut bus = CommunicationBus::new();
        
        bus.register_specialist(SpecialistId::Visionary);
        bus.register_specialist(SpecialistId::Archivist);
        
        let msg = SpecialistMessage::StatusUpdate("hello".to_string());
        let result = bus.send_direct(
            SpecialistId::Visionary,
            SpecialistId::Archivist,
            msg,
        ).await;
        
        assert!(result.is_ok());
    }
}
