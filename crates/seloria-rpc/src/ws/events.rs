use seloria_core::{ClaimStatus, Hash, PublicKey, Vote};
use seloria_vm::ExecutionEvent;
use serde::Serialize;
use tokio::sync::broadcast;
use tracing::debug;

/// WebSocket event types
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum WsEvent {
    BlockCommitted {
        height: u64,
        hash: String,
        tx_count: usize,
        timestamp: u64,
    },
    TxApplied {
        hash: String,
        sender: String,
        success: bool,
    },
    ClaimCreated {
        id: String,
        claim_type: String,
        creator: String,
        stake: u64,
    },
    AttestAdded {
        claim_id: String,
        attester: String,
        vote: String,
        stake: u64,
    },
    ClaimFinalized {
        id: String,
        status: String,
        yes_stake: u64,
        no_stake: u64,
    },
    KvUpdated {
        ns_id: String,
        key: String,
    },
}

impl WsEvent {
    pub fn block_committed(height: u64, hash: Hash, tx_count: usize, timestamp: u64) -> Self {
        WsEvent::BlockCommitted {
            height,
            hash: hash.to_hex(),
            tx_count,
            timestamp,
        }
    }

    pub fn tx_applied(hash: Hash, sender: PublicKey, success: bool) -> Self {
        WsEvent::TxApplied {
            hash: hash.to_hex(),
            sender: sender.to_hex(),
            success,
        }
    }

    pub fn claim_created(id: Hash, claim_type: &str, creator: PublicKey, stake: u64) -> Self {
        WsEvent::ClaimCreated {
            id: id.to_hex(),
            claim_type: claim_type.to_string(),
            creator: creator.to_hex(),
            stake,
        }
    }

    pub fn attest_added(claim_id: Hash, attester: PublicKey, vote: Vote, stake: u64) -> Self {
        let vote_str = match vote {
            Vote::Yes => "yes",
            Vote::No => "no",
        };
        WsEvent::AttestAdded {
            claim_id: claim_id.to_hex(),
            attester: attester.to_hex(),
            vote: vote_str.to_string(),
            stake,
        }
    }

    pub fn claim_finalized(id: Hash, status: ClaimStatus, yes_stake: u64, no_stake: u64) -> Self {
        let status_str = match status {
            ClaimStatus::Pending => "pending",
            ClaimStatus::FinalizedYes => "finalized_yes",
            ClaimStatus::FinalizedNo => "finalized_no",
        };
        WsEvent::ClaimFinalized {
            id: id.to_hex(),
            status: status_str.to_string(),
            yes_stake,
            no_stake,
        }
    }

    pub fn kv_updated(ns_id: Hash, key: &str) -> Self {
        WsEvent::KvUpdated {
            ns_id: ns_id.to_hex(),
            key: key.to_string(),
        }
    }

    pub fn from_execution_event(event: &ExecutionEvent) -> Option<Self> {
        match event {
            ExecutionEvent::ClaimCreated {
                claim_id,
                claim_type,
                creator,
                stake,
            } => Some(WsEvent::claim_created(
                *claim_id,
                claim_type,
                *creator,
                *stake,
            )),
            ExecutionEvent::AttestationAdded {
                claim_id,
                attester,
                vote,
                stake,
            } => Some(WsEvent::attest_added(*claim_id, *attester, *vote, *stake)),
            ExecutionEvent::ClaimFinalized {
                claim_id,
                status,
                yes_stake,
                no_stake,
            } => Some(WsEvent::claim_finalized(
                *claim_id,
                *status,
                *yes_stake,
                *no_stake,
            )),
            ExecutionEvent::KvUpdated { ns_id, key } => Some(WsEvent::kv_updated(*ns_id, key)),
            ExecutionEvent::KvDeleted { ns_id, key } => Some(WsEvent::kv_updated(*ns_id, key)),
            _ => None,
        }
    }
}

/// Event broadcaster for WebSocket clients
pub struct EventBroadcaster {
    sender: broadcast::Sender<WsEvent>,
}

impl EventBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        EventBroadcaster { sender }
    }

    /// Broadcast an event to all connected clients
    pub fn broadcast(&self, event: WsEvent) {
        match self.sender.send(event.clone()) {
            Ok(count) => {
                debug!("Broadcast {:?} to {} clients", event, count);
            }
            Err(_) => {
                // No receivers, that's okay
            }
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.sender.subscribe()
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seloria_core::{hash_blake3, KeyPair};

    #[tokio::test]
    async fn test_broadcast() {
        let broadcaster = EventBroadcaster::new(100);
        let mut rx = broadcaster.subscribe();

        let event = WsEvent::block_committed(1, hash_blake3(b"test"), 5, 1000);
        broadcaster.broadcast(event);

        let received = rx.recv().await.unwrap();
        match received {
            WsEvent::BlockCommitted { height, .. } => assert_eq!(height, 1),
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let broadcaster = EventBroadcaster::new(100);
        let mut rx1 = broadcaster.subscribe();
        let mut rx2 = broadcaster.subscribe();

        assert_eq!(broadcaster.subscriber_count(), 2);

        let event = WsEvent::kv_updated(hash_blake3(b"ns"), "key");
        broadcaster.broadcast(event);

        rx1.recv().await.unwrap();
        rx2.recv().await.unwrap();
    }
}
