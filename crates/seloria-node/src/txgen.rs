use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};

use seloria_core::{
    compute_pool_id, hash_blake3, AgentCertificate, Capability, Hash, KvValue, NamespacePolicy, Op,
    PublicKey, SecretKey, SignedAgentCertificate, TokenMeta, Transaction, Vote,
};

use crate::cli::TxGenCommands;

pub fn handle_txgen(command: TxGenCommands) -> Result<()> {
    match command {
        TxGenCommands::AgentCert {
            issuer_secret,
            agent_secret,
            issued_at,
            expires_at,
            capabilities,
            metadata_hash,
            nonce,
            fee,
            out,
        } => {
            let issuer_secret = parse_secret(&issuer_secret)?;
            let issuer_pub = issuer_secret.public_key();
            let agent_secret = parse_secret(&agent_secret)?;
            let agent_pub = agent_secret.public_key();

            let issuer_id = hash_blake3(issuer_pub.as_bytes());
            let caps = parse_capabilities(&capabilities)?;
            let metadata_hash = metadata_hash
                .as_deref()
                .map(parse_hash)
                .transpose()?
                .unwrap_or(Hash::ZERO);

            let cert = AgentCertificate::new(
                issuer_id,
                agent_pub,
                issued_at,
                expires_at,
                caps,
                metadata_hash,
            );
            let signed = SignedAgentCertificate::new(cert, &issuer_secret)?;

            let tx = Transaction::new_signed(
                agent_pub,
                nonce,
                fee,
                vec![Op::AgentCertRegister { cert: signed }],
                &agent_secret,
            )?;

            write_tx(&tx, &out)?;
        }

        TxGenCommands::Transfer {
            from_secret,
            to_pubkey,
            amount,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();
            let to_pub = parse_pubkey(&to_pubkey)?;

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::Transfer { to: to_pub, amount }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
        }

        TxGenCommands::TokenCreate {
            from_secret,
            name,
            symbol,
            decimals,
            total_supply,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();

            let meta = TokenMeta::new(
                name.clone(),
                symbol.clone(),
                decimals,
                total_supply,
                sender_pub,
            );

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::TokenCreate {
                    name,
                    symbol,
                    decimals,
                    total_supply,
                }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
            println!("TOKEN_ID={}", meta.token_id.to_hex());
        }

        TxGenCommands::TokenTransfer {
            from_secret,
            token_id,
            to_pubkey,
            amount,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();
            let token_id = parse_hash(&token_id)?;
            let to_pub = parse_pubkey(&to_pubkey)?;

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::TokenTransfer {
                    token_id,
                    to: to_pub,
                    amount,
                }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
        }

        TxGenCommands::ClaimCreate {
            from_secret,
            claim_type,
            payload,
            payload_hex,
            stake,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();
            let payload_bytes = parse_bytes(&payload, payload_hex)?;
            let payload_hash = hash_blake3(&payload_bytes);

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::ClaimCreate {
                    claim_type,
                    payload_hash,
                    stake,
                }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
        }

        TxGenCommands::Attest {
            from_secret,
            claim_id,
            vote,
            stake,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();
            let claim_id = parse_hash(&claim_id)?;
            let vote = parse_vote(&vote)?;

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::Attest {
                    claim_id,
                    vote,
                    stake,
                }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
        }

        TxGenCommands::NamespaceCreate {
            from_secret,
            ns_id,
            policy,
            allowlist,
            min_write_stake,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();
            let ns_id = parse_hash(&ns_id)?;
            let policy = parse_policy(&policy)?;
            let allowlist = parse_pubkey_list(&allowlist)?;

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::NamespaceCreate {
                    ns_id,
                    policy,
                    allowlist,
                    min_write_stake,
                }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
        }

        TxGenCommands::KvPut {
            from_secret,
            ns_id,
            key,
            codec,
            value,
            value_hex,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();
            let ns_id = parse_hash(&ns_id)?;
            let value_bytes = parse_bytes(&value, value_hex)?;
            let kv_value = KvValue::inline(&codec, value_bytes);

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::KvPut {
                    ns_id,
                    key,
                    value: kv_value,
                }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
        }

        TxGenCommands::KvDel {
            from_secret,
            ns_id,
            key,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();
            let ns_id = parse_hash(&ns_id)?;

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::KvDel { ns_id, key }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
        }

        TxGenCommands::KvAppend {
            from_secret,
            ns_id,
            key,
            codec,
            value,
            value_hex,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();
            let ns_id = parse_hash(&ns_id)?;
            let value_bytes = parse_bytes(&value, value_hex)?;
            let kv_value = KvValue::inline(&codec, value_bytes);

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::KvAppend {
                    ns_id,
                    key,
                    value: kv_value,
                }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
        }

        TxGenCommands::PoolCreate {
            from_secret,
            token_a,
            token_b,
            amount_a,
            amount_b,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();
            let token_a = parse_hash(&token_a)?;
            let token_b = parse_hash(&token_b)?;

            let pool_id = compute_pool_id(token_a, token_b);

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::PoolCreate {
                    token_a,
                    token_b,
                    amount_a,
                    amount_b,
                }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
            println!("POOL_ID={}", pool_id.to_hex());
        }

        TxGenCommands::PoolAdd {
            from_secret,
            pool_id,
            amount_a,
            amount_b,
            min_lp,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();
            let pool_id = parse_hash(&pool_id)?;

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::PoolAdd {
                    pool_id,
                    amount_a,
                    amount_b,
                    min_lp,
                }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
        }

        TxGenCommands::PoolRemove {
            from_secret,
            pool_id,
            lp_amount,
            min_a,
            min_b,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();
            let pool_id = parse_hash(&pool_id)?;

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::PoolRemove {
                    pool_id,
                    lp_amount,
                    min_a,
                    min_b,
                }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
        }

        TxGenCommands::Swap {
            from_secret,
            pool_id,
            token_in,
            amount_in,
            min_out,
            nonce,
            fee,
            out,
        } => {
            let sender_secret = parse_secret(&from_secret)?;
            let sender_pub = sender_secret.public_key();
            let pool_id = parse_hash(&pool_id)?;
            let token_in = parse_hash(&token_in)?;

            let tx = Transaction::new_signed(
                sender_pub,
                nonce,
                fee,
                vec![Op::Swap {
                    pool_id,
                    token_in,
                    amount_in,
                    min_out,
                }],
                &sender_secret,
            )?;
            write_tx(&tx, &out)?;
        }
    }

    Ok(())
}

fn write_tx(tx: &Transaction, out: &PathBuf) -> Result<()> {
    let json = serde_json::to_string_pretty(tx)?;
    std::fs::write(out, json)?;
    Ok(())
}

fn parse_secret(hex_str: &str) -> Result<SecretKey> {
    SecretKey::from_hex(hex_str).map_err(|e| anyhow!(e))
}

fn parse_pubkey(hex_str: &str) -> Result<PublicKey> {
    PublicKey::from_hex(hex_str).map_err(|e| anyhow!(e))
}

fn parse_hash(hex_str: &str) -> Result<Hash> {
    Hash::from_hex(hex_str).map_err(|e| anyhow!(e))
}

fn parse_capabilities(input: &str) -> Result<Vec<Capability>> {
    let mut caps = Vec::new();
    for part in input.split(',') {
        let cap = match part.trim().to_lowercase().as_str() {
            "txsubmit" | "tx_submit" => Capability::TxSubmit,
            "claim" => Capability::Claim,
            "attest" => Capability::Attest,
            "kvwrite" | "kv_write" => Capability::KvWrite,
            "" => continue,
            other => bail!("Unknown capability '{}'", other),
        };
        caps.push(cap);
    }

    if caps.is_empty() {
        bail!("At least one capability is required");
    }

    Ok(caps)
}

fn parse_vote(input: &str) -> Result<Vote> {
    match input.trim().to_lowercase().as_str() {
        "yes" => Ok(Vote::Yes),
        "no" => Ok(Vote::No),
        other => bail!("Unknown vote '{}'", other),
    }
}

fn parse_policy(input: &str) -> Result<NamespacePolicy> {
    match input.trim().to_lowercase().as_str() {
        "owneronly" | "owner_only" => Ok(NamespacePolicy::OwnerOnly),
        "allowlist" => Ok(NamespacePolicy::Allowlist),
        "stakegated" | "stake_gated" => Ok(NamespacePolicy::StakeGated),
        other => bail!("Unknown policy '{}'", other),
    }
}

fn parse_pubkey_list(input: &str) -> Result<Vec<PublicKey>> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut list = Vec::new();
    for item in input.split(',') {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        list.push(parse_pubkey(trimmed)?);
    }
    Ok(list)
}

fn parse_bytes(input: &str, is_hex: bool) -> Result<Vec<u8>> {
    if is_hex {
        Ok(hex::decode(input)?)
    } else {
        Ok(input.as_bytes().to_vec())
    }
}
