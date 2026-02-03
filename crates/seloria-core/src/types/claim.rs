use serde::{Deserialize, Serialize};

use crate::crypto::{Hash, PublicKey};

/// Vote direction for attestations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    Yes,
    No,
}

/// Status of a claim
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimStatus {
    /// Still accepting attestations
    Pending,
    /// Finalized with YES consensus
    FinalizedYes,
    /// Finalized with NO consensus
    FinalizedNo,
}

/// An attestation on a claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    /// Public key of the attester
    pub attester: PublicKey,
    /// Vote direction
    pub vote: Vote,
    /// Stake amount locked for this attestation
    pub stake: u64,
    /// Block height when attestation was made
    pub block_height: u64,
}

/// A claim that agents can attest to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    /// Unique claim ID
    pub id: Hash,
    /// Type of claim (e.g., "price", "event", "verification")
    pub claim_type: String,
    /// Hash of the claim payload (actual data stored off-chain)
    pub payload_hash: Hash,
    /// Public key of the claim creator
    pub creator: PublicKey,
    /// Stake amount locked by the creator
    pub creator_stake: u64,
    /// Total stake voting YES
    pub yes_stake: u64,
    /// Total stake voting NO
    pub no_stake: u64,
    /// Current status
    pub status: ClaimStatus,
    /// Block height when created
    pub created_at: u64,
    /// List of attestations
    pub attestations: Vec<Attestation>,
}

impl Claim {
    /// Create a new pending claim
    pub fn new(
        id: Hash,
        claim_type: String,
        payload_hash: Hash,
        creator: PublicKey,
        creator_stake: u64,
        created_at: u64,
    ) -> Self {
        Claim {
            id,
            claim_type,
            payload_hash,
            creator,
            creator_stake,
            yes_stake: creator_stake, // Creator implicitly votes YES
            no_stake: 0,
            status: ClaimStatus::Pending,
            created_at,
            attestations: Vec::new(),
        }
    }

    /// Add an attestation to the claim
    pub fn add_attestation(&mut self, attestation: Attestation) {
        match attestation.vote {
            Vote::Yes => self.yes_stake += attestation.stake,
            Vote::No => self.no_stake += attestation.stake,
        }
        self.attestations.push(attestation);
    }

    /// Check if claim should finalize based on stake thresholds
    /// YES finalizes if: yes_stake >= 2 * creator_stake
    /// NO finalizes if: no_stake >= 2 * creator_stake
    pub fn check_finality(&self) -> Option<ClaimStatus> {
        let threshold = 2 * self.creator_stake;

        if self.yes_stake >= threshold {
            Some(ClaimStatus::FinalizedYes)
        } else if self.no_stake >= threshold {
            Some(ClaimStatus::FinalizedNo)
        } else {
            None
        }
    }

    /// Finalize the claim if threshold is met
    pub fn try_finalize(&mut self) -> bool {
        if let Some(final_status) = self.check_finality() {
            self.status = final_status;
            true
        } else {
            false
        }
    }

    /// Check if a specific public key has already attested
    pub fn has_attested(&self, pubkey: &PublicKey) -> bool {
        self.attestations.iter().any(|a| a.attester == *pubkey)
    }

    /// Get the winning and losing side after finalization
    /// Returns (winners, losers) where each is a list of (pubkey, stake, vote)
    pub fn get_settlement_sides(&self) -> Option<(Vec<(&PublicKey, u64)>, Vec<(&PublicKey, u64)>)> {
        match self.status {
            ClaimStatus::Pending => None,
            ClaimStatus::FinalizedYes => {
                let winners: Vec<_> = std::iter::once((&self.creator, self.creator_stake))
                    .chain(
                        self.attestations
                            .iter()
                            .filter(|a| a.vote == Vote::Yes)
                            .map(|a| (&a.attester, a.stake)),
                    )
                    .collect();
                let losers: Vec<_> = self
                    .attestations
                    .iter()
                    .filter(|a| a.vote == Vote::No)
                    .map(|a| (&a.attester, a.stake))
                    .collect();
                Some((winners, losers))
            }
            ClaimStatus::FinalizedNo => {
                let winners: Vec<_> = self
                    .attestations
                    .iter()
                    .filter(|a| a.vote == Vote::No)
                    .map(|a| (&a.attester, a.stake))
                    .collect();
                let losers: Vec<_> = std::iter::once((&self.creator, self.creator_stake))
                    .chain(
                        self.attestations
                            .iter()
                            .filter(|a| a.vote == Vote::Yes)
                            .map(|a| (&a.attester, a.stake)),
                    )
                    .collect();
                Some((winners, losers))
            }
        }
    }
}

/// Settlement parameters
pub const SLASH_PERCENTAGE: u64 = 20; // Losers lose 20% of stake

/// Calculate settlement amounts
/// Returns a map of pubkey -> balance change (positive for gains, negative for losses)
pub fn calculate_settlement(claim: &Claim) -> Option<Vec<(PublicKey, i64)>> {
    let (winners, losers) = claim.get_settlement_sides()?;

    if losers.is_empty() {
        // No losers, just return stakes to everyone
        let mut changes = Vec::new();
        for (pubkey, stake) in winners {
            changes.push((*pubkey, stake as i64));
        }
        return Some(changes);
    }

    // Calculate total slashed from losers
    let total_slashed: u64 = losers
        .iter()
        .map(|(_, stake)| stake * SLASH_PERCENTAGE / 100)
        .sum();

    // Calculate total winning stake for pro-rata distribution
    let total_winning_stake: u64 = winners.iter().map(|(_, stake)| *stake).sum();

    let mut changes = Vec::new();

    // Losers lose 20% of their stake
    for (pubkey, stake) in &losers {
        let slashed = stake * SLASH_PERCENTAGE / 100;
        let returned = stake - slashed;
        changes.push((**pubkey, returned as i64));
    }

    // Winners get their stake back plus pro-rata share of slashed amount
    for (pubkey, stake) in &winners {
        let share = if total_winning_stake > 0 {
            (total_slashed * *stake) / total_winning_stake
        } else {
            0
        };
        changes.push((**pubkey, (*stake + share) as i64));
    }

    Some(changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{hash_blake3, KeyPair};

    fn create_test_claim() -> (Claim, KeyPair) {
        let creator = KeyPair::generate();
        let claim_id = hash_blake3(b"test claim");
        let claim = Claim::new(
            claim_id,
            "test".to_string(),
            hash_blake3(b"payload"),
            creator.public,
            1000,
            1,
        );
        (claim, creator)
    }

    #[test]
    fn test_claim_creation() {
        let (claim, creator) = create_test_claim();
        assert_eq!(claim.creator, creator.public);
        assert_eq!(claim.creator_stake, 1000);
        assert_eq!(claim.yes_stake, 1000); // Creator implicitly votes YES
        assert_eq!(claim.no_stake, 0);
        assert_eq!(claim.status, ClaimStatus::Pending);
    }

    #[test]
    fn test_attestation() {
        let (mut claim, _) = create_test_claim();
        let attester = KeyPair::generate();

        claim.add_attestation(Attestation {
            attester: attester.public,
            vote: Vote::No,
            stake: 500,
            block_height: 2,
        });

        assert_eq!(claim.yes_stake, 1000);
        assert_eq!(claim.no_stake, 500);
        assert!(claim.has_attested(&attester.public));
    }

    #[test]
    fn test_finality_yes() {
        let (mut claim, _) = create_test_claim();
        let attester = KeyPair::generate();

        // Need 2x creator_stake (2000) in YES votes
        // Creator already has 1000
        claim.add_attestation(Attestation {
            attester: attester.public,
            vote: Vote::Yes,
            stake: 1000,
            block_height: 2,
        });

        assert!(claim.try_finalize());
        assert_eq!(claim.status, ClaimStatus::FinalizedYes);
    }

    #[test]
    fn test_finality_no() {
        let (mut claim, _) = create_test_claim();

        // Need 2x creator_stake (2000) in NO votes
        for i in 0..2 {
            let attester = KeyPair::generate();
            claim.add_attestation(Attestation {
                attester: attester.public,
                vote: Vote::No,
                stake: 1000,
                block_height: i + 2,
            });
        }

        assert!(claim.try_finalize());
        assert_eq!(claim.status, ClaimStatus::FinalizedNo);
    }

    #[test]
    fn test_settlement_calculation() {
        let (mut claim, _) = create_test_claim();

        // Add enough NO votes to finalize as NO
        let attester1 = KeyPair::generate();
        let attester2 = KeyPair::generate();

        claim.add_attestation(Attestation {
            attester: attester1.public,
            vote: Vote::No,
            stake: 1000,
            block_height: 2,
        });
        claim.add_attestation(Attestation {
            attester: attester2.public,
            vote: Vote::No,
            stake: 1000,
            block_height: 3,
        });

        claim.try_finalize();
        assert_eq!(claim.status, ClaimStatus::FinalizedNo);

        let settlement = calculate_settlement(&claim).unwrap();
        // Losers (creator) should lose 20% of 1000 = 200
        // Winners (attester1, attester2) split the 200
        // Each winner gets their stake back + 100

        let creator_change = settlement
            .iter()
            .find(|(pk, _)| *pk == claim.creator)
            .map(|(_, c)| *c)
            .unwrap();
        assert_eq!(creator_change, 800); // Lost 20%

        let winner1_change = settlement
            .iter()
            .find(|(pk, _)| *pk == attester1.public)
            .map(|(_, c)| *c)
            .unwrap();
        assert_eq!(winner1_change, 1100); // Stake + 50% of 200
    }
}
