use seloria_core::{hash_blake3, PublicKey, SignedAgentCertificate};
use seloria_state::{ChainState, Storage};
use tracing::debug;

use crate::error::VmError;

/// Execute AGENT_CERT_REGISTER operation
pub fn execute_agent_cert_register<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    cert: &SignedAgentCertificate,
    current_time: u64,
) -> Result<(), VmError> {
    if &cert.cert.agent_pubkey != sender {
        return Err(VmError::InvalidOperation(
            "Sender must match certificate agent pubkey".to_string(),
        ));
    }

    // 1. Compute issuer public key from issuer_id
    // For now, we check if there's a trusted issuer whose pubkey hashes to issuer_id
    let issuer_pubkey = find_issuer_by_id(state, &cert.cert.issuer_id)?;

    // 2. Verify the issuer signature
    cert.verify_signature(&issuer_pubkey)?;

    // 3. Check issuer is trusted
    if !state.is_trusted_issuer(&issuer_pubkey) {
        return Err(VmError::IssuerNotTrusted(issuer_pubkey.to_hex()));
    }

    // 4. Check certificate is not expired
    if cert.is_expired(current_time) {
        return Err(VmError::InvalidOperation(
            "Certificate is expired".to_string(),
        ));
    }

    // 5. Register the agent
    state.register_agent(cert.clone());

    debug!(
        "Registered agent {} from issuer {}",
        cert.cert.agent_pubkey, issuer_pubkey
    );

    Ok(())
}

/// Find a trusted issuer by their ID (hash of pubkey)
fn find_issuer_by_id<S: Storage>(
    state: &ChainState<S>,
    issuer_id: &seloria_core::Hash,
) -> Result<PublicKey, VmError> {
    for issuer in &state.trusted_issuers {
        let computed_id = hash_blake3(issuer.as_bytes());
        if computed_id == *issuer_id {
            return Ok(*issuer);
        }
    }
    Err(VmError::IssuerNotTrusted(issuer_id.to_hex()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use seloria_core::{AgentCertificate, Capability, Hash, KeyPair};
    use seloria_state::MemoryStorage;

    #[test]
    fn test_register_agent_cert() {
        let mut state = ChainState::new(MemoryStorage::new());
        let issuer = KeyPair::generate();
        let agent = KeyPair::generate();

        // Add issuer as trusted
        state.trusted_issuers.insert(issuer.public);

        // Create certificate
        let issuer_id = hash_blake3(issuer.public.as_bytes());
        let cert = AgentCertificate::new(
            issuer_id,
            agent.public,
            0,
            1000,
            vec![Capability::TxSubmit],
            Hash::ZERO,
        );
        let signed_cert = SignedAgentCertificate::new(cert, &issuer.secret).unwrap();

        // Register
        execute_agent_cert_register(&mut state, &agent.public, &signed_cert, 500).unwrap();

        // Verify registration
        assert!(state.get_agent(&agent.public).is_some());
        assert!(state.is_certified_agent(&agent.public, 500));
    }

    #[test]
    fn test_register_untrusted_issuer() {
        let mut state = ChainState::new(MemoryStorage::new());
        let untrusted_issuer = KeyPair::generate();
        let agent = KeyPair::generate();

        // Don't add issuer as trusted

        let issuer_id = hash_blake3(untrusted_issuer.public.as_bytes());
        let cert = AgentCertificate::new(
            issuer_id,
            agent.public,
            0,
            1000,
            vec![Capability::TxSubmit],
            Hash::ZERO,
        );
        let signed_cert = SignedAgentCertificate::new(cert, &untrusted_issuer.secret).unwrap();

        let result = execute_agent_cert_register(&mut state, &agent.public, &signed_cert, 500);
        assert!(matches!(result, Err(VmError::IssuerNotTrusted(_))));
    }

    #[test]
    fn test_register_expired_cert() {
        let mut state = ChainState::new(MemoryStorage::new());
        let issuer = KeyPair::generate();
        let agent = KeyPair::generate();

        state.trusted_issuers.insert(issuer.public);

        let issuer_id = hash_blake3(issuer.public.as_bytes());
        let cert = AgentCertificate::new(
            issuer_id,
            agent.public,
            0,
            100, // Expires at 100
            vec![Capability::TxSubmit],
            Hash::ZERO,
        );
        let signed_cert = SignedAgentCertificate::new(cert, &issuer.secret).unwrap();

        let result = execute_agent_cert_register(&mut state, &agent.public, &signed_cert, 500); // Current time 500
        assert!(matches!(result, Err(VmError::InvalidOperation(_))));
    }
}
