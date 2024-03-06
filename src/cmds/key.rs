use anyhow::anyhow;
use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::bip32::{DerivationPath, ExtendedPrivKey, KeySource};
use bdk::bitcoin::Network;
use bdk::descriptor::Segwitv0;
use bdk::keys::bip39::{Language, Mnemonic, WordCount};
use bdk::keys::DescriptorKey::Secret;
use bdk::keys::{DerivableKey, DescriptorKey, ExtendedKey, GeneratableKey, GeneratedKey};
use bdk::miniscript::miniscript;
use clap::Subcommand;
use serde_json::json;

#[derive(Debug, Subcommand, Clone, PartialEq, Eq)]
pub enum KeySubCommand {
    /// Generates new random seed mnemonic phrase and corresponding master extended key.
    Generate {
        /// Entropy level based on number of random seed mnemonic words. (allowed values: [12, 24])
        #[clap(
            name = "WORD_COUNT",
            short = 'e',
            long = "entropy",
            default_value = "24"
        )]
        word_count: usize,
        /// Seed password.
        #[clap(name = "PASSWORD", short = 'p', long = "password")]
        password: Option<String>,
    },
    /// Restore a master extended key from seed backup mnemonic words.
    Restore {
        /// Seed mnemonic words, must be quoted (eg. "word1 word2 ...").
        #[clap(name = "MNEMONIC", short = 'm', long = "mnemonic")]
        mnemonic: String,
        /// Seed password.
        #[clap(name = "PASSWORD", short = 'p', long = "password")]
        password: Option<String>,
    },
    /// Derive a child key pair from a master extended key and a derivation path string (eg. "m/84'/1'/0'/0" or "m/84h/1h/0h/0").
    Derive {
        /// Extended private key to derive from.
        #[clap(name = "XPRV", short = 'x', long = "xprv")]
        xprv: ExtendedPrivKey,
        /// Path to use to derive extended public key from extended private key.
        #[clap(name = "PATH", short = 'p', long = "path")]
        path: DerivationPath,
    },
}

pub fn handle_key_subcommand(
    network: Network,
    subcommand: KeySubCommand,
) -> Result<serde_json::Value, anyhow::Error> {
    let secp = Secp256k1::new();
    match subcommand {
        KeySubCommand::Generate {
            word_count,
            password,
        } => {
            let mnemonic_type = match word_count {
                12 => WordCount::Words12,
                _ => WordCount::Words24,
            };
            let mnemonic: GeneratedKey<_, miniscript::BareCtx> =
                Mnemonic::generate((mnemonic_type, Language::English))
                    .map_err(|_| anyhow!("Mnemonic generation error"))?;
            let mnemonic = mnemonic.into_key();
            let xkey: ExtendedKey = (mnemonic.clone(), password).into_extended_key()?;
            let xprv = xkey
                .into_xprv(network)
                .ok_or_else(|| anyhow!("Privatekey info not found (should not happen)"))?;
            let fingerprint = xprv.fingerprint(&secp);
            let phrase = mnemonic
                .word_iter()
                .fold("".to_string(), |phrase, w| phrase + w + " ")
                .trim()
                .to_string();
            Ok(
                json!({ "mnemonic": phrase, "xprv": xprv.to_string(), "fingerprint": fingerprint.to_string() }),
            )
        }
        KeySubCommand::Restore { mnemonic, password } => {
            let mnemonic = Mnemonic::parse_in(Language::English, mnemonic)?;
            let xkey: ExtendedKey = (mnemonic, password).into_extended_key()?;
            let xprv = xkey
                .into_xprv(network)
                .ok_or_else(|| anyhow!("Privatekey info not found (should not happen)"))?;
            let fingerprint = xprv.fingerprint(&secp);

            Ok(json!({ "xprv": xprv.to_string(), "fingerprint": fingerprint.to_string() }))
        }

        KeySubCommand::Derive { xprv, path } => {
            if xprv.network != network {
                return Err(anyhow!("InvalidNetwork"));
            }
            let derived_xprv = &xprv.derive_priv(&secp, &path)?;

            let origin: KeySource = (xprv.fingerprint(&secp), path);

            let derived_xprv_desc_key: DescriptorKey<Segwitv0> =
                derived_xprv.into_descriptor_key(Some(origin), DerivationPath::default())?;

            if let Secret(desc_seckey, _, _) = derived_xprv_desc_key {
                let desc_pubkey = desc_seckey.to_public(&secp)?;
                Ok(json!({"xpub": desc_pubkey.to_string(), "xprv": desc_seckey.to_string()}))
            } else {
                Err(anyhow!("Invalid key variant"))
            }
        }
    }
}
