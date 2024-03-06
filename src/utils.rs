use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::anyhow;
use bdk::blockchain::{
    AnyBlockchain, AnyBlockchainConfig, ConfigurableBlockchain, ElectrumBlockchainConfig,
};
use bdk::{
    bitcoin::{psbt::Psbt, secp256k1::Secp256k1, Address, Network, OutPoint},
    database::{any::SledDbConfiguration,
         AnyDatabase, AnyDatabaseConfig, BatchDatabase,
        ConfigurableDatabase,
    },
    wallet::wallet_name_from_descriptor,
    Wallet,
};
use crate::opts::WalletOpts;

/// Create a randomized wallet name from the descriptor checksum.
/// If wallet options already includes a name, use that instead.
pub(crate) fn maybe_descriptor_wallet_name(
    wallet_opts: WalletOpts,
    network: Network,
) -> Result<WalletOpts, anyhow::Error> {
    if wallet_opts.wallet.is_some() {
        return Ok(wallet_opts);
    }
    // Use deterministic wallet name derived from descriptor
    let wallet_name = wallet_name_from_descriptor(
        &wallet_opts.descriptor[..],
        wallet_opts.change_descriptor.as_deref(),
        network,
        &Secp256k1::new(),
    )?;
    let mut wallet_opts = wallet_opts;
    wallet_opts.wallet = Some(wallet_name);

    Ok(wallet_opts)
}

#[allow(unused_variables)]
/// Open the wallet database.
pub(crate) fn open_database(
    wallet_opts: &WalletOpts,
    home_path: &Path,
) -> Result<AnyDatabase, anyhow::Error> {
    let wallet_name = wallet_opts.wallet.as_ref().expect("wallet name");
    let database_path = prepare_wallet_db_dir(&wallet_name, home_path)?;
    let config = AnyDatabaseConfig::Sled(SledDbConfiguration {
        path: database_path
            .into_os_string()
            .into_string()
            .expect("path string"),
        tree_name: wallet_name.clone()
    });
    let database = AnyDatabase::from_config(&config)?;
    log::debug!("database opened successfully");
    Ok(database)
}

/// Prepare wallet database directory.
fn prepare_wallet_db_dir(wallet_name: &str, home_path: &Path) -> Result<PathBuf, anyhow::Error> {
    let mut db_dir = prepare_wallet_dir(wallet_name, home_path)?;
    db_dir.push("wallet.sqlite");
    Ok(db_dir)
}

/// Prepare bdk_cli wallet directory.
fn prepare_wallet_dir(wallet_name: &str, home_path: &Path) -> Result<PathBuf, anyhow::Error> {
    let mut dir = home_path.to_owned();
    dir.push(wallet_name);
    if !dir.exists() {
        log::info!("Creating wallet directory {}", dir.as_path().display());
        std::fs::create_dir(&dir)?;
    }
    Ok(dir)
}

/// Create a new blockchain for a given [Nodes] if available
/// or else create one from the wallet configuration options.
pub(crate) fn new_blockchain(wallet_opts: &WalletOpts) -> Result<AnyBlockchain, anyhow::Error> {
    let config = AnyBlockchainConfig::Electrum(ElectrumBlockchainConfig {
        url: wallet_opts.electrum_opts.server.to_owned(),
        socks5: wallet_opts.proxy_opts.proxy.clone(),
        retry: wallet_opts.proxy_opts.retries,
        timeout: wallet_opts.electrum_opts.timeout,
        stop_gap: wallet_opts.electrum_opts.stop_gap,
        validate_domain: true,
    });

    Ok(AnyBlockchain::from_config(&config)?)
}

/// Prepare bdk-cli home directory
///
/// This function is called to check if [`crate::CliOpts`] datadir is set.
/// If not the default home directory is created at `~/.bdk-bitcoin`.
pub(crate) fn prepare_home_dir(home_path: Option<PathBuf>) -> Result<PathBuf, anyhow::Error> {
    let dir = home_path.unwrap_or_else(|| {
        let mut dir = PathBuf::new();
        dir.push(
            &dirs_next::home_dir()
                .ok_or_else(|| anyhow!("home dir not found"))
                .unwrap(),
        );
        dir.push(".bdk-bitcoin");
        dir
    });

    if !dir.exists() {
        log::info!("Creating home directory {}", dir.as_path().display());
        std::fs::create_dir(&dir)?;
    }

    Ok(dir)
}

/// Parse the proxy (Socket:Port) argument from the cli input.
pub(crate) fn parse_proxy_auth(s: &str) -> Result<(String, String), String> {
    let parts: Vec<_> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid format".to_string());
    }

    let user = parts[0].to_string();
    let passwd = parts[1].to_string();

    Ok((user, passwd))
}

/// Parse the recipient (Address,Amount) argument from cli input.
pub(crate) fn parse_recipient(s: &str) -> Result<(bdk::bitcoin::ScriptBuf, u64), String> {
    let parts: Vec<_> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid format".to_string());
    }
    let addr = Address::from_str(parts[0]).map_err(|e| e.to_string())?;
    let val = u64::from_str(parts[1]).map_err(|e| e.to_string())?;

    Ok((addr.payload.script_pubkey(), val))
}

/// Parse a outpoint (Txid:Vout) argument from cli input.
pub(crate) fn parse_outpoint(s: &str) -> Result<OutPoint, String> {
    OutPoint::from_str(s).map_err(|e| e.to_string())
}

/// Create a new wallet from given wallet configuration options.
pub(crate) fn new_wallet<D>(
    network: Network,
    wallet_opts: &WalletOpts,
    database: D,
) -> Result<Wallet<D>, anyhow::Error>
where
    D: BatchDatabase,
{
    let descriptor = wallet_opts.descriptor.as_str();
    let change_descriptor = wallet_opts.change_descriptor.as_deref();
    let wallet = Wallet::new(descriptor, change_descriptor, network, database)?;
    Ok(wallet)
}

/// Determine if PSBT has final script sigs or witnesses for all unsigned tx inputs.
pub(crate) fn is_final(psbt: &Psbt) -> Result<(), anyhow::Error> {
    let unsigned_tx_inputs = psbt.unsigned_tx.input.len();
    let psbt_inputs = psbt.inputs.len();
    if unsigned_tx_inputs != psbt_inputs {
        return Err(anyhow!(
            "Malformed PSBT, {} unsigned tx inputs and {} psbt inputs.",
            unsigned_tx_inputs,
            psbt_inputs
        ));
    }
    let sig_count = psbt.inputs.iter().fold(0, |count, input| {
        if input.final_script_sig.is_some() || input.final_script_witness.is_some() {
            count + 1
        } else {
            count
        }
    });
    if unsigned_tx_inputs > sig_count {
        return Err(anyhow!(
            "The PSBT is not finalized, inputs are are not fully signed."
        ));
    }
    Ok(())
}
