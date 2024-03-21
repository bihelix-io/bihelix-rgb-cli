use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

use amplify::hex::ToHex;
use amplify::Array;
use amplify::{
    confinement::{U16, U24},
    Display,
};
use bdk::bitcoin::Network;
use bdk::electrum_client::ElectrumApi;
use bp::{Outpoint, Vout};
use psbt::{Psbt, PsbtVer, RgbOutExt};
use seals::txout::SealTxid;

use crate::cmds::runtime::Runtime;
use anyhow::anyhow;
use bp::seals::txout::CloseMethod;
use clap::{Subcommand, ValueEnum};
use rgb::validation::Validity;
use rgb::BlindingFactor;
use rgb::OutputSeal;
use rgb_rt::{Resolver, RuntimeError};
use rgb_schemata::{nia_rgb20, nia_schema};
use rgbstd::containers::{Bindle, BindleContent, BuilderSeal, Transfer, UniversalBindle};
use rgbstd::contract::{ContractId, GenesisSeal, GraphSeal, StateType};
use rgbstd::interface::{ContractBuilder, FilterExclude, IfaceId, SchemaIfaces};
use rgbstd::invoice::{Beneficiary, RgbInvoiceBuilder, XChainNet};
use rgbstd::persistence::{Inventory, Stash};
use rgbstd::schema::SchemaId;
use rgbstd::XChain;
use serde_json::Value::Null as JsonNull;
use strict_types::encoding::{FieldName, StrictSerialize, TypeName};
use strict_types::tn;
use strict_types::StrictVal;

/// Rgb operation subcommands

#[derive(ValueEnum, Copy, Clone, Eq, PartialEq, Hash, Debug, Display, Default)]
#[display(lowercase)]
pub enum InspectFormat {
    #[default]
    Yaml,
    Toml,
    Json,
    Debug,
    Contractum,
}

#[derive(Subcommand, Clone, PartialEq, Eq, Debug, Display)]
#[display(lowercase)]
#[allow(clippy::large_enum_variant)]
pub enum RgbSubCommand {
    /// Prints out list of known RGB schemata
    Schemata,
    /// Prints out list of known RGB interfaces
    Interfaces,
    /// Prints out list of known RGB contracts
    Contracts,

    // Set the rgb20 schemata and implementation
    Set,
    // generate random blinding factor
    Blinding,
    /// Imports RGB data into the stash: contracts, schema, interfaces, etc
    #[display("import")]
    Import {
        /// Use BASE64 ASCII armoring for binary data
        #[arg(short)]
        armored: bool,

        /// File with RGB data
        ///
        /// If not provided, assumes `-a` and prints out data to STDOUT
        file: PathBuf,
    },

    /// Exports existing RGB contract
    #[display("export")]
    Export {
        /// Use BASE64 ASCII armoring for binary data
        #[arg(short)]
        armored: bool,

        /// Contract to export
        contract: ContractId,

        /// File with RGB data
        ///
        /// If not provided, assumes `-a` and reads the data from STDIN
        file: Option<PathBuf>,
    },

    /// Reports information about state of a contract
    #[display("state")]
    State {
        /// Show all state - not just the one owned by the wallet
        #[clap(short, long)]
        all: bool,

        /// Contract identifier
        contract_id: ContractId,

        /// Interface to interpret the state data
        iface: String,

        /// address conflict with all
        #[clap(long, conflicts_with = "all")]
        address: Option<String>,
    },

    /// Issues new contract
    #[display("issue")]
    Issue {
        /// Schema name to use for the contract
        schema: SchemaId, //String,

        /// File containing contract genesis description in YAML format
        contract: PathBuf,
    },

    /// Create new invoice
    #[display("invoice")]
    Invoice {
        /// Contract identifier
        contract_id: ContractId,

        /// Interface to interpret the state data
        iface: String,

        /// Value to transfer
        value: u64,
        seal: String,
    },

    /// Inspects any RGB data file
    #[display("inspect")]
    Inspect {
        /// Format used for data inspection
        #[clap(short, long, default_value = "yaml")]
        format: InspectFormat,

        /// RGB file to inspect
        file: PathBuf,
    },

    /// Validate transfer consignment
    #[display("validate")]
    Validate {
        /// File with the transfer consignment
        file: PathBuf,
    },

    /// Validate transfer consignment & accept to the stash
    #[display("accept")]
    Accept {
        /// Force accepting consignments with non-mined terminal witness
        #[arg(short, long)]
        force: bool,

        /// File with the transfer consignment
        file: PathBuf,
    },
    #[display("sethost")]
    SetHost {
        #[clap(long, default_value = "opret1st")]
        /// Method for single-use-seals.
        method: CloseMethod,

        /// PSBT file.
        psbt_file: PathBuf,
    },
}

pub fn handle_rgb_subcommand(
    data_dir: PathBuf,
    chain: &bdk::bitcoin::Network,
    electrum: Option<String>,
    subcommand: RgbSubCommand,
) -> Result<serde_json::Value, anyhow::Error> {
    let electrum = electrum.as_deref().unwrap_or_else(|| match chain {
        Network::Bitcoin => "blockstream.info:110",
        Network::Testnet => "blockstream.info:143",
        _ => {
            eprint!("No electrum server for this network");
            std::process::exit(1);
        }
    });
    let mut resolver = Resolver::new(electrum)?;
    let mut runtime = Runtime::load(data_dir.clone(), chain).map_err(|err| anyhow!("{}", err))?;

    match subcommand {
        RgbSubCommand::Schemata => {
            for id in runtime.schema_ids()? {
                print!("{id} ");
                for iimpl in runtime.schema(id)?.iimpls.values() {
                    let iface = runtime.iface_by_id(iimpl.iface_id)?;
                    print!("{} ", iface.name);
                }
                println!();
            }
            Ok(JsonNull)
        }
        RgbSubCommand::Interfaces => {
            for (id, name) in runtime.ifaces()? {
                println!("{} {id}", name);
            }
            Ok(JsonNull)
        }
        RgbSubCommand::Contracts => {
            for id in runtime.contract_ids()? {
                println!("{id}");
            }
            Ok(JsonNull)
        }
        RgbSubCommand::Blinding => {
            let blinding_factor = BlindingFactor::random().to_hex();
            eprintln!("blinding_factor {blinding_factor}");
            Ok(JsonNull)
        }
        RgbSubCommand::Set => {
            let schema_bindle = nia_schema().bindle();
            eprintln!("schema_bindle {schema_bindle}");

            schema_bindle.save("your_custom_path/demo/rgb-schemata/NonInflatableAssets.rgb")?;
            let iimpl_bindle = nia_rgb20().bindle();
            eprintln!("iimpl_bindle {iimpl_bindle}");

            iimpl_bindle
                .save("your_custom_path/demo/rgb-schemata/NonInflatableAssets-RGB20.rgb")?;
            Ok(JsonNull)
        }
        RgbSubCommand::Import { armored, file } => {
            if armored {
                todo!()
            } else {
                let bindle = UniversalBindle::load_file(file)?;

                match bindle {
                    UniversalBindle::Iface(iface) => {
                        let id = iface.id();
                        let name = iface.name.clone();
                        runtime
                            .import_iface(iface)
                            .map_err(|err| anyhow!("{}", err))?;
                        eprintln!("Interface {id} with name {name} imported to the stash");
                    }
                    UniversalBindle::Schema(schema) => {
                        let id = schema.id();
                        runtime
                            .import_schema(schema)
                            .map_err(|err| anyhow!("{}", err))?;
                        eprintln!("Schema {id} imported to the stash");
                    }
                    UniversalBindle::Impl(iimpl) => {
                        let iface_id = iimpl.iface_id;
                        let schema_id = iimpl.schema_id;
                        let id = iimpl.id();
                        runtime
                            .import_iface_impl(iimpl)
                            .map_err(|err| anyhow!("{}", err))?;
                        eprintln!(
                            "Implementation {id} of interface {iface_id} for schema \
                             {schema_id} imported to the stash"
                        );
                    }
                    UniversalBindle::Contract(bindle) => {
                        let id = bindle.id();
                        let contract =
                            bindle
                                .unbindle()
                                .validate(&mut resolver, true)
                                .map_err(|c| {
                                    anyhow!("{}", c.validation_status().expect("just validated"))
                                })?;
                        runtime
                            .import_contract(contract, &mut resolver)
                            .map_err(|err| anyhow!("{}", err))?;
                        eprintln!("Contract {id} imported to the stash");
                    }
                    UniversalBindle::Transfer(_) => todo!(),
                };
            }
            Ok(JsonNull)
        }

        RgbSubCommand::Export {
            armored,
            contract,
            file,
        } => {
            let bindle = runtime
                .export_contract(contract)
                .map_err(|err| anyhow!("{}", err))?;
            if let Some(file) = file {
                if armored {
                    const PLATE_TITLE: &str = "RGB CONTRACT";
                    let (data, sigs) = bindle.clone().into_split();
                    let mut f = fs::File::create(&file)?;
                    writeln!(f, "-----BEGIN {}-----", PLATE_TITLE)?;
                    writeln!(f, "Id: {:-#}", bindle.id())?;
                    if let Some(mnemonic) = bindle.bindle_mnemonic() {
                        writeln!(f, "Mnemonic: {}", mnemonic)?;
                    }
                    for (header, value) in bindle.bindle_headers() {
                        writeln!(f, "{header}: {value}")?;
                    }
                    for cert in sigs {
                        writeln!(f, "Signed-By: {}", cert.signer)?;
                    }
                    writeln!(f)?;

                    // TODO: Replace with streamed writer
                    let bin = data.to_strict_serialized::<U24>().expect("in-memory");
                    let hex_str = base64::encode(&bin);
                    let mut str_ref = hex_str.as_str();
                    while str_ref.len() >= 64 {
                        let (line, rest) = str_ref.split_at(64);
                        writeln!(f, "{}", line)?;
                        str_ref = rest;
                    }
                    writeln!(f, "{}", str_ref)?;

                    writeln!(f, "\n-----END {}-----", PLATE_TITLE)?;
                } else {
                    bindle.save(&file)?;
                }
                eprintln!("Contract {contract} exported to '{}'", file.display());
            } else {
                println!("{bindle}");
            }
            Ok(JsonNull)
        }

        RgbSubCommand::State {
            all,
            contract_id,
            iface,
            address,
        } => {
            let iface = runtime.iface_by_name(&tn!(iface.to_owned()))?.clone();
            let contract = runtime
                .contract_iface_id(contract_id, iface.iface_id())
                .map_err(|err| anyhow!("{}", err))?;

            println!("Global:");
            for global in &contract.iface.global_state {
                if let Ok(values) = contract.global(global.name.clone()) {
                    for val in values {
                        println!("  {} := {}", global.name, val);
                    }
                }
            }

            let utxo = address.map(|address| {
                let client = bdk::electrum_client::Client::new(&electrum).unwrap();
                client
                    .script_list_unspent(
                        &bdk::bitcoin::Address::from_str(&address)
                            .unwrap()
                            .require_network(*chain)
                            .unwrap()
                            .script_pubkey(),
                    )
                    .unwrap()
                    .into_iter()
                    .map(|x| format!("{}:{}", x.tx_hash, x.tx_pos))
                    .collect::<HashSet<_>>()
            });

            println!("\nOwned:");
            for owned in &contract.iface.assignments {
                println!("  {}:", owned.name);
                if let Some(utxo) = utxo {
                    if let Ok(allocations) = contract.fungible(owned.name.clone(), &runtime) {
                        allocations
                            .iter()
                            .filter(|x| {
                                let seal = x.owner.as_reduced_unsafe();
                                let outpoint = seal.txid.map_to_outpoint(seal.vout).unwrap();
                                utxo.contains(&outpoint.to_string())
                            })
                            .for_each(|x| {
                                println!(
                                    "    amount={}, utxo={}, witness={} # owned by the wallet",
                                    x.value, x.owner, x.witness
                                );
                            });
                    }
                    return Ok(JsonNull);
                }
                if let Ok(allocations) = contract.fungible(owned.name.clone(), &runtime) {
                    for allocation in allocations {
                        println!(
                            "    amount={}, utxo={}, witness={} # owned by the wallet",
                            allocation.value, allocation.owner, allocation.witness
                        );
                    }
                }
                if all {
                    if let Ok(allocations) =
                        contract.fungible(owned.name.clone(), &FilterExclude(&runtime))
                    {
                        for allocation in allocations {
                            println!(
                                "    amount={}, utxo={}, witness={} # owner unknown",
                                allocation.value, allocation.owner, allocation.witness
                            );
                        }
                    }
                }
                // TODO: Print out other types of state
            }
            Ok(JsonNull)
        }
        RgbSubCommand::Issue { schema, contract } => {
            let file = fs::File::open(contract)?;

            let code = serde_yaml::from_reader::<_, serde_yaml::Value>(file)?;

            let code = code
                .as_mapping()
                .expect("invalid YAML root-level structure");

            let iface_name = code
                .get("interface")
                .expect("contract must specify interface under which it is constructed")
                .as_str()
                .expect("interface name must be a string");
            let SchemaIfaces {
                ref schema,
                ref iimpls,
            } = runtime.schema(schema)?;
            let iface_name = tn!(iface_name.to_owned());
            let iface = runtime
                .iface_by_name(&iface_name)
                .or_else(|_| {
                    let id = IfaceId::from_str(iface_name.as_str())?;
                    runtime.iface_by_id(id).map_err(RuntimeError::from)
                })
                .expect("get iface failed")
                .clone();
            let iface_id = iface.iface_id();
            let iface_impl = iimpls
                .get(&iface_id)
                .ok_or_else(|| {
                    RuntimeError::Custom(format!(
                        "no known interface implementation for {iface_name}"
                    ))
                })
                .expect("get iface impl failed");
            let types = &schema.type_system;

            let mut builder =
                ContractBuilder::testnet(iface.clone(), schema.clone(), iface_impl.clone())?;

            if let Some(globals) = code.get("globals") {
                for (name, val) in globals
                    .as_mapping()
                    .expect("invalid YAML: globals must be an mapping")
                {
                    let name = name
                        .as_str()
                        .expect("invalid YAML: global name must be a string");
                    let name = iface
                        .genesis
                        .global
                        .iter()
                        .find(|(n, _)| n.as_str() == name)
                        .and_then(|(_, spec)| spec.name.as_ref())
                        .map(FieldName::as_str)
                        .unwrap_or(name);
                    let state_type = iface_impl
                        .global_state
                        .iter()
                        .find(|info| info.name.as_str() == name)
                        .unwrap_or_else(|| panic!("unknown type name '{name}'"))
                        .id;
                    let sem_id = schema
                        .global_types
                        .get(&state_type)
                        .expect("invalid schema implementation")
                        .sem_id;
                    let val = StrictVal::from(val.clone());
                    let typed_val = types
                        .typify(val, sem_id)
                        .expect("global type doesn't match type definition");

                    let serialized = types
                        .strict_serialize_type::<U16>(&typed_val)
                        .expect("internal error");
                    // Workaround for borrow checker:
                    let field_name =
                        FieldName::try_from(name.to_owned()).expect("invalid type name");
                    builder = builder
                        .add_global_state(field_name, serialized)
                        .expect("invalid global state data");
                }
            }

            if let Some(assignments) = code.get("assignments") {
                for (name, val) in assignments
                    .as_mapping()
                    .expect("invalid YAML: assignments must be an mapping")
                {
                    let name = name
                        .as_str()
                        .expect("invalid YAML: assignments name must be a string");
                    let name = iface
                        .genesis
                        .assignments
                        .iter()
                        .find(|(n, _)| n.as_str() == name)
                        .and_then(|(_, spec)| spec.name.as_ref())
                        .map(FieldName::as_str)
                        .unwrap_or(name);
                    let state_type = iface_impl
                        .assignments
                        .iter()
                        .find(|info| info.name.as_str() == name)
                        .expect("unknown type name")
                        .id;
                    let state_schema = schema
                        .owned_types
                        .get(&state_type)
                        .expect("invalid schema implementation");

                    let assign = val.as_mapping().expect("an assignment must be a mapping");
                    let seal = assign
                        .get("seal")
                        .expect("assignment doesn't provide seal information")
                        .as_str()
                        .expect("seal must be a string");

                    let seal = OutputSeal::from_str(seal).expect("invalid seal definition");
                    let seal = GenesisSeal::new_random(seal.method, seal.txid, seal.vout);

                    // Workaround for borrow checker:
                    let field_name =
                        FieldName::try_from(name.to_owned()).expect("invalid type name");
                    match state_schema.state_type() {
                        StateType::Void => todo!(),
                        StateType::Fungible => {
                            let amount = assign
                                .get("amount")
                                .expect("owned state must be a fungible amount")
                                .as_u64()
                                .expect("fungible state must be an integer");
                            let seal = BuilderSeal::Revealed(XChain::Bitcoin(seal));
                            builder = builder
                                .add_fungible_state(field_name, seal, amount)
                                .expect("invalid global state data");
                        }
                        StateType::Structured => todo!(),
                        StateType::Attachment => todo!(),
                    }
                }
            }

            let contract = builder.issue_contract().expect("failure issuing contract");
            let id = contract.contract_id();
            let validated_contract = contract
                .validate(&mut resolver, true)
                .map_err(|consignment| {
                    RuntimeError::IncompleteContract(
                        consignment
                            .into_validation_status()
                            .expect("just validated"),
                    )
                })
                .unwrap();
            runtime
                .import_contract(validated_contract, &mut resolver)
                .expect("failure importing issued contract");
            eprintln!(
                "A new contract {id} is issued and added to the stash.\nUse `export` command \
                     to export the contract."
            );
            Ok(JsonNull)
        }
        RgbSubCommand::Inspect { file, format } => {
            let bindle = UniversalBindle::load_file(file)?;
            // TODO: For now, serde implementation doesn't work for consignments due to
            //       some of the keys which can't be serialized to strings. Once this fixed,
            //       allow this inspect formats option
            let s = match format {
                InspectFormat::Yaml => {
                    serde_yaml::to_string(&bindle).expect("unable to present as YAML")
                }
                InspectFormat::Toml => toml::to_string(&bindle).expect("unable to present as TOML"),
                InspectFormat::Json => {
                    serde_json::to_string(&bindle).expect("unable to present as JSON")
                }
                InspectFormat::Debug => format!("{bindle:#?}"),
                InspectFormat::Contractum => todo!("contractum representation"),
            };
            println!("{s}");
            Ok(JsonNull)
        }

        RgbSubCommand::Validate { file } => {
            let bindle = Bindle::<Transfer>::load_file(file)?;
            let consignment = bindle.unbindle();
            resolver.add_terminals(&consignment);
            let status = match consignment.validate(&mut resolver, true) {
                Ok(consignment) => consignment.into_validation_status(),
                Err(consignment) => consignment.into_validation_status(),
            }
            .expect("just validated");
            if status.validity() == Validity::Valid {
                eprintln!("The provided consignment is valid")
            } else {
                eprintln!("{status}");
            }
            Ok(JsonNull)
        }
        RgbSubCommand::Accept { force, file } => {
            let bindle = Bindle::<Transfer>::load_file(file)?;
            let consignment = bindle.unbindle();
            resolver.add_terminals(&consignment);
            let transfer = consignment
                .validate(&mut resolver, true)
                .unwrap_or_else(|c| c);
            eprintln!("{}", transfer.validation_status().expect("just validated"));
            runtime
                .accept_transfer(transfer, &mut resolver, force)
                .unwrap();
            eprintln!("Transfer accepted into the stash");
            Ok(JsonNull)
        }
        RgbSubCommand::SetHost { psbt_file, method } => {
            let psbt_data = fs::read(&psbt_file)?;
            let mut psbt = Psbt::deserialize(&psbt_data)?;
            let mut psbt_modified = false;
            match method {
                CloseMethod::OpretFirst => {
                    psbt.to_unsigned_tx()
                        .outputs
                        .iter()
                        .zip(&mut psbt.outputs_mut())
                        .find(|(o, outp)| o.script_pubkey.is_op_return() && !outp.is_opret_host())
                        .and_then(|(_, outp)| {
                            psbt_modified = true;
                            outp.set_rgb_velocity_hint(
                                rgbstd::interface::VelocityHint::Unspecified,
                            );
                            outp.set_opret_host().ok()
                        });
                }
                CloseMethod::TapretFirst => {
                    psbt.to_unsigned_tx()
                        .outputs
                        .iter()
                        .zip(&mut psbt.outputs_mut())
                        .find(|(o, outp)| o.script_pubkey.is_p2tr() && !outp.is_tapret_host())
                        .and_then(|(_, outp)| {
                            psbt_modified = true;
                            outp.set_tapret_host().ok()
                        });
                }
            };
            fs::write(&psbt_file, psbt.serialize(PsbtVer::V0))?;

            if psbt_modified {
                eprintln!(
                    "PSBT file '{}' is updated with {method} host now set.",
                    psbt_file.display()
                );
            }
            Ok(JsonNull)
        }
        RgbSubCommand::Invoice {
            contract_id,
            iface,
            value,
            seal,
        } => {
            let iface = TypeName::try_from(iface.to_owned()).expect("invalid interface name");
            let network = bpstd::Network::Testnet3;
            let mut output_vec = seal.split(":");
            let txid_str = output_vec.next().unwrap();
            let vout_str = output_vec.next().unwrap();

            let output = Outpoint::new(
                Array::from_str(txid_str).unwrap().into(),
                Vout::from_str(vout_str).unwrap(),
            );
            let graph_seal = XChain::Bitcoin(GraphSeal::opret_first_rand_from(output));
            runtime
                .store_seal_secret(graph_seal)
                .expect("store seal secret error");
            let beneficiary =
                Beneficiary::BlindedSeal(*graph_seal.to_secret_seal().as_reduced_unsafe());

            let invoice = RgbInvoiceBuilder::new(XChainNet::bitcoin(network, beneficiary))
                .set_contract(contract_id)
                .set_interface(iface)
                .set_amount_raw(value)
                .finish();
            println!("{invoice}");
            Ok(JsonNull)
        }
    }
}
