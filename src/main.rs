mod cmds;
mod opts;
mod utils;

use std::collections::BTreeMap;
use std::fs;

use crate::cmds::{key::handle_key_subcommand, rgb::handle_rgb_subcommand};
use crate::opts::{
    Cli, Command, OfflineWalletSubCommand, OnlineWalletSubCommand, WalletOpts, WalletSubCommand,
};
use crate::utils::{
    is_final, maybe_descriptor_wallet_name, new_blockchain, new_wallet, open_database,
    prepare_home_dir,
};
use anyhow::anyhow;
use base64::{decode, encode};
use bdk::bitcoin::psbt::Psbt;
use bdk::bitcoin::script::PushBytesBuf;
use bdk::bitcoin::Network;
use bdk::wallet::tx_builder::TxOrdering;
use bdk::wallet::AddressIndex;
use bdk::{
    blockchain::{log_progress, Blockchain},
    database::BatchDatabase,
    Error as BdkError, FeeRate, KeychainKind, Wallet,
};
use bdk::{SignOptions, SyncOptions};
use clap::Parser;
use serde_json::{json, Value as JsonValue};

// 1040 / 630 = 1.65
pub const FEE_FACTOR: f32 = 1.65;

fn main() {
    env_logger::init();
    let cli = Cli::parse();
    log::debug!("cli: {:?}", cli);
    match handle_command(cli) {
        Ok(JsonValue::Null) => {}
        Ok(value) => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
        Err(err) => {
            eprintln!("ERROR: {}", err);
            std::process::exit(-1);
        }
    }
}

pub fn handle_command(cli: Cli) -> Result<JsonValue, anyhow::Error> {
    let network = cli.network;
    match cli.command {
        Command::ExtractPsbt { path } => {
            let psbt: Psbt = Psbt::deserialize(&fs::read(path)?)?;
            Ok(serde_json::to_value(psbt).unwrap())
        }
        Command::Key { subcommand } => handle_key_subcommand(network, subcommand),
        Command::Wallet {
            datadir,
            wallet_opts,
            subcommand: WalletSubCommand::OnlineWalletSubCommand(online_subcommand),
        } => {
            let home_dir = prepare_home_dir(datadir)?;
            let wallet_opts = maybe_descriptor_wallet_name(wallet_opts, cli.network)?;
            let database = open_database(&wallet_opts, &home_dir)?;
            let blockchain = new_blockchain(&wallet_opts)?;
            let wallet = new_wallet(network, &wallet_opts, database)?;
            handle_online_wallet_subcommand(&wallet, &blockchain, online_subcommand)
        }
        Command::Wallet {
            datadir,
            wallet_opts,
            subcommand: WalletSubCommand::OfflineWalletSubCommand(offline_subcommand),
        } => {
            let home_dir = prepare_home_dir(datadir)?;
            let wallet_opts = maybe_descriptor_wallet_name(wallet_opts, cli.network)?;
            let database = open_database(&wallet_opts, &home_dir)?;
            log::info!("wallet_opts: {:?}", wallet_opts);
            let wallet = new_wallet(network, &wallet_opts, database)?;
            handle_offline_wallet_subcommand(&wallet, &wallet_opts, offline_subcommand)
        }
        Command::Rgb {
            data_dir,
            electrum,
            subcommand,
        } => {
            let chain = match network {
                Network::Regtest => Network::Regtest,
                Network::Testnet => Network::Testnet,
                Network::Bitcoin => Network::Bitcoin,
                Network::Signet => Network::Signet,
                _ => todo!(),
            };
            handle_rgb_subcommand(data_dir, &chain, electrum, subcommand)

            // handle_rgb_subcommand(data_dir, electrum, subcommand)
        }
    }
}

pub(crate) fn handle_online_wallet_subcommand<B, D>(
    wallet: &Wallet<D>,
    blockchain: &B,
    online_subcommand: OnlineWalletSubCommand,
) -> Result<serde_json::Value, anyhow::Error>
where
    B: Blockchain,
    D: BatchDatabase,
{
    match online_subcommand {
        OnlineWalletSubCommand::Sync => {
            wallet.sync(
                blockchain,
                SyncOptions {
                    progress: Some(Box::new(log_progress())),
                },
            )?;
            Ok(json!("done"))
        }
        OnlineWalletSubCommand::Broadcast { psbt_file } => {
            let psbt: Psbt = Psbt::deserialize(&fs::read(psbt_file)?)?;
            is_final(&psbt)?;
            let tx = psbt.extract_tx();
            blockchain.broadcast(&tx)?;
            Ok(json!({
                "txid": tx.txid(),
                "tx.weight": tx.weight(),
                "tx.size": tx.size(),
                "tx.vsize": tx.vsize(),
            }))
        }
    }
}

pub fn handle_offline_wallet_subcommand<D>(
    wallet: &Wallet<D>,
    wallet_opts: &WalletOpts,
    offline_subcommand: OfflineWalletSubCommand,
) -> Result<serde_json::Value, anyhow::Error>
where
    D: BatchDatabase,
{
    match offline_subcommand {
        OfflineWalletSubCommand::GetNewAddress => {
            let addr = wallet.get_address(AddressIndex::New)?;
            if wallet_opts.verbose {
                Ok(json!({"address": addr.address, "index": addr.index}))
            } else {
                Ok(json!({"address": addr.address}))
            }
        }
        OfflineWalletSubCommand::ListUnspent => Ok(serde_json::to_value(&wallet.list_unspent()?)?),
        OfflineWalletSubCommand::CreateTx {
            recipients,
            send_all,
            enable_rbf,
            offline_signer,
            utxos,
            unspendable,
            fee_rate,
            external_policy,
            internal_policy,
            add_data,
            add_string,
            psbt_file,
        } => {
            let mut tx_builder = wallet.build_tx();

            if send_all {
                tx_builder.drain_wallet().drain_to(recipients[0].0.clone());
            } else {
                tx_builder.set_recipients(recipients);
            }

            if enable_rbf {
                tx_builder.enable_rbf();
            }

            if offline_signer {
                tx_builder.include_output_redeem_witness_script();
            }

            if let Some(fee_rate) = fee_rate {
                tx_builder.fee_rate(FeeRate::from_sat_per_vb(fee_rate));
            }

            if let Some(utxos) = utxos {
                tx_builder.add_utxos(&utxos[..])?.manually_selected_only();
            }
            if let Some(unspendable) = unspendable {
                tx_builder.unspendable(unspendable);
            }

            if let Some(base64_data) = add_data {
                let op_return_data = decode(&base64_data).expect("get op return data failed");
                let push_bytes = PushBytesBuf::try_from(op_return_data).unwrap();
                tx_builder.add_data(&push_bytes);
            } else if let Some(string_data) = add_string {
                let push_bytes = PushBytesBuf::try_from(string_data.as_bytes().to_vec())
                    .expect("add string failed");

                tx_builder.add_data(&push_bytes);
            }

            let policies = vec![
                external_policy.map(|p| (p, KeychainKind::External)),
                internal_policy.map(|p| (p, KeychainKind::Internal)),
            ];

            for (policy, keychain) in policies.into_iter().flatten() {
                let policy = serde_json::from_str::<BTreeMap<String, Vec<usize>>>(&policy)?;
                tx_builder.policy_path(policy, keychain);
            }

            let (psbt, details) = tx_builder.finish()?;
            fs::write(psbt_file, psbt.serialize())?;
            if wallet_opts.verbose {
                Ok(json!({"details": details, "psbt": psbt}))
            } else {
                Ok(json!({"details": details}))
            }
        }
        OfflineWalletSubCommand::Sign {
            psbt_file,
            assume_height,
            trust_witness_utxo,
        } => {
            let mut psbt: Psbt = Psbt::deserialize(&fs::read(&psbt_file)?)?;

            // if psbt.inputs.len() != 2 {
            //     return Err(anyhow!(
            //         "invalid inputs length: {}, expected: 2",
            //         psbt.inputs.len()
            //     ));
            // }
            // if psbt.outputs.len() != 3 {
            //     return Err(anyhow!(
            //         "invalid outputs length: {}, expected: 3",
            //         psbt.outputs.len()
            //     ));
            // }

            let signopt = SignOptions {
                assume_height,
                trust_witness_utxo: trust_witness_utxo.unwrap_or(false),
                ..Default::default()
            };
            let finalized = wallet.sign(&mut psbt, signopt)?;
            fs::write(psbt_file, psbt.serialize())?;
            if wallet_opts.verbose {
                Ok(json!({"is_finalized": finalized, "psbt": psbt}))
            } else {
                Ok(json!({"is_finalized": finalized}))
            }
        }

        OfflineWalletSubCommand::InitPayfeePsbt {
            input,
            output,
            fee_rate,
            psbt_file,
        } => {
            let mut tx_builder = wallet.build_tx();
            tx_builder.ordering(TxOrdering::Untouched);
            tx_builder.drain_wallet().drain_to(output.0.clone());

            if let Some(fee_rate) = fee_rate {
                let actual_fee_rate = fee_rate * FEE_FACTOR;
                tx_builder.fee_rate(FeeRate::from_sat_per_vb(actual_fee_rate));
            }

            tx_builder.add_utxo(input)?.manually_selected_only();

            // --add_string opret
            let push_bytes = PushBytesBuf::try_from("opret".as_bytes().to_vec()).unwrap();
            tx_builder.add_data(&push_bytes);
            let (psbt, details) = tx_builder.finish()?;

            // TODO: optimize
            // psbt.inputs[0].sighash_type = Some(EcdsaSighashType::Single.into());

            let psbt_bin = psbt.serialize();
            //  serialize(&psbt);
            fs::write(psbt_file, &psbt_bin)?;
            if wallet_opts.verbose {
                Ok(json!({
                    "psbt": encode(psbt_bin),
                    "details": details,
                    "serialized_psbt": psbt,
                }))
            } else {
                Ok(json!({
                    "psbt": encode(psbt_bin),
                    "details": details,
                }))
            }
        }
        OfflineWalletSubCommand::AddSenderInput {
            input,
            fee_rate,
            enable_rbf,
            psbt_file,
        } => {
            let old_psbt: Psbt = Psbt::deserialize(&fs::read(&psbt_file)?)?;

            if old_psbt.inputs.len() != 1 || old_psbt.unsigned_tx.input.len() != 1 {
                return Err(anyhow!(
                    "invalid psbt inputs, inputs.len={} != 1",
                    old_psbt.inputs.len()
                ));
            }
            if old_psbt.outputs.len() != 2 || old_psbt.unsigned_tx.output.len() != 2 {
                return Err(anyhow!(
                    "invalid psbt outputs, outputs.len={} != 2",
                    old_psbt.outputs.len()
                ));
            }
            // FIXME: add more checks

            let input_txout = wallet.get_utxo(input)?.ok_or(BdkError::UnknownUtxo)?.txout;
            let recipients = [
                old_psbt.unsigned_tx.output[0].clone(),
                old_psbt.unsigned_tx.output[1].clone(),
                input_txout,
            ]
            .into_iter()
            .map(|txout| (txout.script_pubkey, txout.value))
            .collect::<Vec<_>>();

            let mut tx_builder = wallet.build_tx();
            tx_builder.ordering(TxOrdering::Untouched);
            tx_builder.add_foreign_utxo(
                old_psbt.unsigned_tx.input[0].previous_output,
                old_psbt.inputs[0].clone(),
                0,
            )?;
            tx_builder.add_utxo(input)?;
            tx_builder.set_recipients(recipients);

            if let Some(fee_rate) = fee_rate {
                tx_builder.fee_rate(FeeRate::from_sat_per_vb(fee_rate));
            }

            if enable_rbf {
                tx_builder.enable_rbf();
            }

            let (psbt, details) = tx_builder.finish()?;
            let psbt_bin = psbt.serialize();
            // serialize(&psbt);
            fs::write(&psbt_file, &psbt_bin)?;
            if wallet_opts.verbose {
                Ok(json!({
                    "psbt": encode(psbt_bin),
                    "details": details,
                    "serialized_psbt": psbt,
                }))
            } else {
                Ok(json!({
                    "psbt": encode(psbt_bin),
                    "details": details,
                }))
            }
        }
    }
}
