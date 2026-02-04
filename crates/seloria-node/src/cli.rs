use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Seloria - An agent-only blockchain
#[derive(Parser)]
#[command(name = "seloria")]
#[command(about = "Seloria node and utilities")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a Seloria node
    Run {
        /// Path to configuration file
        #[arg(short, long, default_value = "config.json")]
        config: PathBuf,
    },

    /// Initialize a new node configuration
    Init {
        /// Output path for configuration file
        #[arg(short, long, default_value = "config.json")]
        output: PathBuf,

        /// Generate sample keypairs
        #[arg(long, default_value = "true")]
        with_keys: bool,
    },

    /// Generate a new keypair
    Keygen {
        /// Output file for secret key
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Show node status
    Status {
        /// RPC endpoint
        #[arg(short, long, default_value = "http://127.0.0.1:8080")]
        endpoint: String,
    },

    /// Submit a transaction
    Tx {
        /// RPC endpoint
        #[arg(short, long, default_value = "http://127.0.0.1:8080")]
        endpoint: String,

        /// Transaction JSON file
        #[arg(short, long)]
        file: PathBuf,
    },

    /// Generate signed transactions for testing
    Txgen {
        #[command(subcommand)]
        command: TxGenCommands,
    },

    /// Snapshot utilities
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommands,
    },
}

#[derive(Subcommand)]
pub enum SnapshotCommands {
    /// Download a snapshot from a node
    Pull {
        /// RPC endpoint
        #[arg(short, long, default_value = "http://127.0.0.1:8080")]
        endpoint: String,
        /// Output file path
        #[arg(short, long, default_value = "seloria-data/state.bin")]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum TxGenCommands {
    /// Create an AGENT_CERT_REGISTER transaction
    AgentCert {
        /// Issuer secret key hex
        #[arg(long)]
        issuer_secret: String,
        /// Agent secret key hex (tx sender)
        #[arg(long)]
        agent_secret: String,
        /// Issued at (unix seconds)
        #[arg(long)]
        issued_at: u64,
        /// Expires at (unix seconds)
        #[arg(long)]
        expires_at: u64,
        /// Capabilities (comma-separated: txsubmit,claim,attest,kvwrite)
        #[arg(long)]
        capabilities: String,
        /// Metadata hash hex (optional, defaults to zero hash)
        #[arg(long)]
        metadata_hash: Option<String>,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create a TRANSFER transaction
    Transfer {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Recipient public key hex
        #[arg(long)]
        to_pubkey: String,
        /// Amount to send
        #[arg(long)]
        amount: u64,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create a TOKEN_CREATE transaction
    TokenCreate {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Token name
        #[arg(long)]
        name: String,
        /// Token symbol
        #[arg(long)]
        symbol: String,
        /// Token decimals
        #[arg(long)]
        decimals: u8,
        /// Total supply
        #[arg(long)]
        total_supply: u64,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create a TOKEN_TRANSFER transaction
    TokenTransfer {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Token ID hex
        #[arg(long)]
        token_id: String,
        /// Recipient public key hex
        #[arg(long)]
        to_pubkey: String,
        /// Amount to send
        #[arg(long)]
        amount: u64,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create a CLAIM_CREATE transaction
    ClaimCreate {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Claim type string
        #[arg(long)]
        claim_type: String,
        /// Payload string (or hex if --payload-hex is set)
        #[arg(long)]
        payload: String,
        /// Treat payload as hex bytes
        #[arg(long, default_value = "false")]
        payload_hex: bool,
        /// Stake amount
        #[arg(long)]
        stake: u64,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create an ATTEST transaction
    Attest {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Claim ID hex
        #[arg(long)]
        claim_id: String,
        /// Vote (yes/no)
        #[arg(long)]
        vote: String,
        /// Stake amount
        #[arg(long)]
        stake: u64,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create a NAMESPACE_CREATE transaction
    NamespaceCreate {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Namespace ID hex
        #[arg(long)]
        ns_id: String,
        /// Policy (owneronly, allowlist, stakegated)
        #[arg(long)]
        policy: String,
        /// Allowlist pubkeys (comma-separated hex)
        #[arg(long, default_value = "")]
        allowlist: String,
        /// Minimum write stake for stake-gated policy
        #[arg(long, default_value = "0")]
        min_write_stake: u64,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create a KV_PUT transaction (inline data)
    KvPut {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Namespace ID hex
        #[arg(long)]
        ns_id: String,
        /// Key string
        #[arg(long)]
        key: String,
        /// Codec (e.g., json, raw)
        #[arg(long)]
        codec: String,
        /// Value string (or hex if --value-hex is set)
        #[arg(long)]
        value: String,
        /// Treat value as hex bytes
        #[arg(long, default_value = "false")]
        value_hex: bool,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create a KV_DEL transaction
    KvDel {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Namespace ID hex
        #[arg(long)]
        ns_id: String,
        /// Key string
        #[arg(long)]
        key: String,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create a KV_APPEND transaction (inline data)
    KvAppend {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Namespace ID hex
        #[arg(long)]
        ns_id: String,
        /// Key string
        #[arg(long)]
        key: String,
        /// Codec (e.g., json, raw)
        #[arg(long)]
        codec: String,
        /// Value string (or hex if --value-hex is set)
        #[arg(long)]
        value: String,
        /// Treat value as hex bytes
        #[arg(long, default_value = "false")]
        value_hex: bool,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create a POOL_CREATE transaction
    PoolCreate {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Token A ID hex
        #[arg(long)]
        token_a: String,
        /// Token B ID hex
        #[arg(long)]
        token_b: String,
        /// Amount of token A
        #[arg(long)]
        amount_a: u64,
        /// Amount of token B
        #[arg(long)]
        amount_b: u64,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create a POOL_ADD transaction
    PoolAdd {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Pool ID hex
        #[arg(long)]
        pool_id: String,
        /// Amount of token A
        #[arg(long)]
        amount_a: u64,
        /// Amount of token B
        #[arg(long)]
        amount_b: u64,
        /// Minimum LP tokens to mint
        #[arg(long)]
        min_lp: u64,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create a POOL_REMOVE transaction
    PoolRemove {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Pool ID hex
        #[arg(long)]
        pool_id: String,
        /// LP amount to burn
        #[arg(long)]
        lp_amount: u64,
        /// Minimum amount of token A
        #[arg(long)]
        min_a: u64,
        /// Minimum amount of token B
        #[arg(long)]
        min_b: u64,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Create a SWAP transaction
    Swap {
        /// Sender secret key hex
        #[arg(long)]
        from_secret: String,
        /// Pool ID hex
        #[arg(long)]
        pool_id: String,
        /// Token input ID hex
        #[arg(long)]
        token_in: String,
        /// Amount in
        #[arg(long)]
        amount_in: u64,
        /// Minimum amount out
        #[arg(long)]
        min_out: u64,
        /// Transaction nonce
        #[arg(long)]
        nonce: u64,
        /// Transaction fee
        #[arg(long)]
        fee: u64,
        /// Output file (JSON)
        #[arg(short, long)]
        out: PathBuf,
    },
}
