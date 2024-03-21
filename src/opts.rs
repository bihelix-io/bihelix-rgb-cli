#![allow(clippy::large_enum_variant)]

use std::path::PathBuf;

use bdk::bitcoin::{Network, OutPoint, ScriptBuf};
use clap::{Args, Parser, Subcommand, ValueHint};

use crate::cmds::{key::KeySubCommand, rgb::RgbSubCommand};
use crate::utils::{parse_outpoint, parse_proxy_auth, parse_recipient};

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
pub const RGB_DATA_DIR: &str = "~/.rgb";
#[cfg(target_os = "macos")]
pub const RGB_DATA_DIR: &str = "~/Library/Application Support/RGB Smart Contracts";
#[cfg(target_os = "windows")]
pub const RGB_DATA_DIR: &str = "~\\AppData\\Local\\RGB Smart Contracts";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    pub verbose: bool,

    /// Sets the network.
    #[clap(
        name = "NETWORK",
        short = 'n',
        long = "network",
        default_value = "testnet"
    )]
    pub network: Network,

    /// Command to execute.
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Extract info from psbt file
    ExtractPsbt {
        /// The PSBT file path (binary format)
        #[clap(short, long)]
        path: PathBuf,
    },

    /// Execute key commands.
    Key {
        #[clap(subcommand)]
        subcommand: KeySubCommand,
    },

    /// Wallet operations.
    ///
    /// bdk-cli wallet operations includes all the basic wallet level tasks.
    /// Most commands can be used without connecting to any backend. To use commands that
    /// needs backend like `sync` and `broadcast`, compile the binary with specific backend feature
    /// and use the configuration options below to configure for that backend.
    Wallet {
        /// Sets the wallet data directory.
        /// Default value : "~/.bdk-bitcoin
        #[clap(name = "DATADIR", short = 'w', long = "walletdir")]
        datadir: Option<PathBuf>,
        #[clap(flatten)]
        wallet_opts: WalletOpts,
        #[clap(subcommand)]
        subcommand: WalletSubCommand,
    },

    /// RGB operations
    Rgb {
        /// Data directory path.
        ///
        /// Path to the directory that contains RGB stored data.
        #[clap(
            short = 'd',
            long,
            default_value = RGB_DATA_DIR,
            value_hint = ValueHint::DirPath
        )]
        data_dir: PathBuf,

        /// Electrum server to use.
        #[clap(short = 's', long)]
        electrum: Option<String>,

        #[clap(subcommand)]
        subcommand: RgbSubCommand,
    },
}

/// Wallet operation subcommands.
#[derive(Debug, Subcommand, Clone, PartialEq)]
pub enum WalletSubCommand {
    #[clap(flatten)]
    OnlineWalletSubCommand(OnlineWalletSubCommand),
    #[clap(flatten)]
    OfflineWalletSubCommand(OfflineWalletSubCommand),
}

/// Wallet subcommands that needs a blockchain backend.
#[derive(Debug, Subcommand, Clone, PartialEq)]
pub enum OnlineWalletSubCommand {
    /// Syncs with the chosen blockchain server.
    Sync,
    /// Broadcasts a transaction to the network. Takes a PSBT to extract.
    Broadcast {
        /// Sets the PSBT file to broadcast.
        #[clap(name = "PSBT", long = "psbt")]
        psbt_file: PathBuf,
    },
}

/// Wallet subcommands that can be issued without a blockchain backend.
#[derive(Debug, Subcommand, Clone, PartialEq)]
pub enum OfflineWalletSubCommand {
    /// Generates a new external address.
    GetNewAddress,
    /// Lists the available spendable UTXOs.
    ListUnspent,
    /// Creates a new unsigned transaction.
    CreateTx {
        /// Adds a recipient to the transaction.
        // Clap Doesn't support complex vector parsing https://github.com/clap-rs/clap/issues/1704.
        // Address and amount parsing is done at run time in handler function.
        #[clap(name = "ADDRESS:SAT", long = "to", required = true, value_parser = parse_recipient)]
        recipients: Vec<(ScriptBuf, u64)>,
        /// Sends all the funds (or all the selected utxos). Requires only one recipient with value 0.
        #[clap(long = "send_all", short = 'a')]
        send_all: bool,
        /// Enables Replace-By-Fee (BIP125).
        #[clap(long = "enable_rbf", short = 'r')]
        enable_rbf: bool,
        /// Make a PSBT that can be signed by offline signers and hardware wallets. Forces the addition of `non_witness_utxo` and more details to let the signer identify the change output.
        #[clap(long = "offline_signer")]
        offline_signer: bool,
        /// Selects which utxos *must* be spent.
        #[clap(name = "MUST_SPEND_TXID:VOUT", long = "utxos", value_parser = parse_outpoint)]
        utxos: Option<Vec<OutPoint>>,
        /// Marks a utxo as unspendable.
        #[clap(name = "CANT_SPEND_TXID:VOUT", long = "unspendable", value_parser = parse_outpoint)]
        unspendable: Option<Vec<OutPoint>>,
        /// Fee rate to use in sat/vbyte.
        #[clap(name = "SATS_VBYTE", short = 'f', long = "fee_rate")]
        fee_rate: Option<f32>,
        /// Selects which policy should be used to satisfy the external descriptor.
        #[clap(name = "EXT_POLICY", long = "external_policy")]
        external_policy: Option<String>,
        /// Selects which policy should be used to satisfy the internal descriptor.
        #[clap(name = "INT_POLICY", long = "internal_policy")]
        internal_policy: Option<String>,
        /// Optionally create an OP_RETURN output containing given String in utf8 encoding (max 80 bytes)
        #[clap(
            name = "ADD_STRING",
            long = "add_string",
            short = 's',
            conflicts_with = "ADD_DATA"
        )]
        add_string: Option<String>,
        /// Optionally create an OP_RETURN output containing given base64 encoded String. (max 80 bytes)
        #[clap(
            name = "ADD_DATA",
            long = "add_data",
            short = 'o',
            conflicts_with = "ADD_STRING"
        )]
        add_data: Option<String>, //base 64 econding

        /// The output PSBT file path
        #[clap(long = "psbt")]
        psbt_file: PathBuf,
    },
    /// Signs and tries to finalize a PSBT.
    Sign {
        /// The output PSBT file path
        #[clap(long = "psbt")]
        psbt_file: PathBuf,

        /// Assume the blockchain has reached a specific height. This affects the transaction finalization, if there are timelocks in the descriptor.
        #[clap(name = "HEIGHT", long = "assume_height")]
        assume_height: Option<u32>,
        /// Whether the signer should trust the witness_utxo, if the non_witness_utxo hasn’t been provided.
        #[clap(name = "WITNESS", long = "trust_witness_utxo")]
        trust_witness_utxo: Option<bool>,
    },

    /// Receiver init a Partially Signed Bitcoin Transaction (PSBT) for pay the fee
    InitPayfeePsbt {
        /// Selects which utxo *must* be spent. (Do NOT have RGB20 token)
        #[clap(name = "TXID:VOUT", long = "input", required = true, value_parser = parse_outpoint)]
        input: OutPoint,

        /// Adds a recipient to the transaction.
        #[clap(name = "ADDRESS:SAT", long = "to", required = true, value_parser = parse_recipient)]
        output: (ScriptBuf, u64),

        /// Fee rate to use in sat/vbyte. (actual_fee_rate = fee_rate * 2)
        #[clap(name = "SATS_VBYTE", short = 'f', long = "fee_rate")]
        fee_rate: Option<f32>,

        /// The output PSBT file path
        #[clap(long = "psbt")]
        psbt_file: PathBuf,
    },
    /// Add sender input (and add a corresponding output)
    AddSenderInput {
        /// Selects which utxo *must* be spent.(Have RGB20 token)
        #[clap(name = "TXID:VOUT", long = "input", required = true, value_parser = parse_outpoint)]
        input: OutPoint,

        /// Fee rate to use in sat/vbyte.
        #[clap(name = "SATS_VBYTE", short = 'f', long = "fee_rate")]
        fee_rate: Option<f32>,

        /// Enables Replace-By-Fee (BIP125).
        #[clap(long = "enable_rbf", short = 'r')]
        enable_rbf: bool,

        /// The PSBT file path
        #[clap(long = "psbt")]
        psbt_file: PathBuf,
    },
}

/// Config options wallet operations can take.
#[derive(Debug, Parser, Clone, PartialEq, Eq)]
pub struct WalletOpts {
    /// Selects the wallet to use.
    #[clap(name = "WALLET_NAME", short = 'w', long = "wallet")]
    pub wallet: Option<String>,
    /// Adds verbosity, returns PSBT in JSON format alongside serialized, displays expanded objects.
    #[clap(name = "VERBOSE", short = 'v', long = "verbose")]
    pub verbose: bool,
    /// Sets the descriptor to use for the external addresses.
    #[clap(name = "DESCRIPTOR", short = 'd', long = "descriptor", required = true)]
    pub descriptor: String,
    /// Sets the descriptor to use for internal addresses.
    #[clap(name = "CHANGE_DESCRIPTOR", short = 'c', long = "change_descriptor")]
    pub change_descriptor: Option<String>,
    #[clap(flatten)]
    pub electrum_opts: ElectrumOpts,
    #[clap(flatten)]
    pub proxy_opts: ProxyOpts,
}

/// Options to configure electrum backend.
#[derive(Debug, Args, Clone, PartialEq, Eq)]
pub struct ElectrumOpts {
    /// Sets the SOCKS5 proxy timeout for the Electrum client.
    #[clap(name = "PROXY_TIMEOUT", short = 't', long = "timeout")]
    pub timeout: Option<u8>,
    /// Sets the Electrum server to use.
    #[clap(
        name = "ELECTRUM_URL",
        short = 's',
        long = "server",
        default_value = "ssl://electrum.blockstream.info:60002"
    )]
    pub server: String,

    /// Stop searching addresses for transactions after finding an unused gap of this length.
    #[clap(
        name = "STOP_GAP",
        long = "stop_gap",
        short = 'g',
        default_value = "10"
    )]
    pub stop_gap: usize,
}

/// Options to configure a SOCKS5 proxy for a blockchain client connection.
#[derive(Debug, Args, Clone, PartialEq, Eq)]
pub struct ProxyOpts {
    /// Sets the SOCKS5 proxy for a blockchain client.
    #[clap(name = "PROXY_ADDRS:PORT", long = "proxy", short = 'p')]
    pub proxy: Option<String>,

    /// Sets the SOCKS5 proxy credential.
    #[clap(name="PROXY_USER:PASSWD", long="proxy_auth", short='a', value_parser = parse_proxy_auth)]
    pub proxy_auth: Option<(String, String)>,

    /// Sets the SOCKS5 proxy retries for the blockchain client.
    #[clap(
        name = "PROXY_RETRIES",
        short = 'r',
        long = "retries",
        default_value = "5"
    )]
    pub retries: u8,
}
