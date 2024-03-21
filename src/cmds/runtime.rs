// RGB smart contracts for Bitcoin & Lightning
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2019-2023 by
//     Dr Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2019-2023 LNP/BP Standards Association. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use rgbfs::StockFs;
use rgbstd::containers::{Contract, LoadError, Transfer};
use rgbstd::interface::{BuilderError, OutpointFilter};
use rgbstd::persistence::{Inventory, InventoryDataError, InventoryError, StashError, Stock};
use std::convert::Infallible;
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

use rgbstd::resolvers::ResolveHeight;
use rgbstd::validation;
use rgbstd::validation::ResolveWitness;
use rgbstd::XOutpoint;

use strict_types::encoding::{DeserializeError, Ident, SerializeError};

use amplify::{Display, Error, From, Getters};

#[derive(Debug, Display, Error, From)]
#[display(inner)]
pub enum RuntimeError {
    #[from]
    Io(io::Error),

    #[from]
    Yaml(serde_yaml::Error),

    #[from]
    Serialize(SerializeError),

    #[from]
    Deserialize(DeserializeError),

    #[from]
    Load(LoadError),

    #[from]
    Stash(StashError<Infallible>),

    #[from]
    #[from(InventoryDataError<Infallible>)]
    Inventory(InventoryError<Infallible>),

    #[from]
    Builder(BuilderError),

    /// wallet with id '{0}' is not known to the system
    #[display(doc_comments)]
    WalletUnknown(Ident),

    #[from]
    Psbt(bitcoin::psbt::Error),

    #[cfg(feature = "electrum")]
    #[from]
    Electrum(electrum_client::Error),

    #[from]
    InvalidConsignment(validation::Status),

    /// the contract source doesn't provide all state information required by
    /// the schema. This means that some of the global fields or assignments are
    /// missed.
    #[display(doc_comments)]
    IncompleteContract,

    #[from]
    Custom(String),
}

impl From<Infallible> for RuntimeError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Getters)]
pub struct Runtime {
    #[getter(skip)]
    stock: Stock,
}

impl Deref for Runtime {
    type Target = Stock;
    fn deref(&self) -> &Self::Target {
        &self.stock
    }
}

impl DerefMut for Runtime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stock
    }
}
impl OutpointFilter for Runtime {
    fn include_output(&self, _output: impl Into<XOutpoint>) -> bool {
        true
    }
}
use std::fs;
#[allow(clippy::result_large_err)]
impl Runtime {
    pub fn load(
        mut data_dir: PathBuf,
        chain: &bdk::bitcoin::Network,
    ) -> Result<Self, RuntimeError> {
        data_dir.push(chain.to_string());
        #[cfg(feature = "log")]
        debug!("Using data directory '{}'", data_dir.display());
        fs::create_dir_all(&data_dir)?;

        let mut stock_path = data_dir.clone();
        stock_path.push("stock.dat");
        #[cfg(feature = "log")]
        debug!("Reading stock from '{}'", stock_path.display());
        let stock = if !stock_path.exists() {
            #[cfg(feature = "log")]
            info!("Stock file not found, creating default stock");
            #[cfg(feature = "cli")]
            eprintln!("Stock file not found, creating default stock");
            let stock = Stock::default();
            stock.store(&stock_path)?;
            stock
        } else {
            Stock::load(&stock_path)?
        };

        let mut wallets_path = data_dir.clone();
        wallets_path.push("wallets.yml");

        Ok(Self { stock })
    }

    pub fn import_contract<R: ResolveHeight>(
        &mut self,
        contract: Contract,
        resolver: &mut R,
    ) -> Result<validation::Status, RuntimeError>
    where
        R::Error: 'static,
    {
        self.stock
            .import_contract(contract, resolver)
            .map_err(RuntimeError::from)
    }

    pub fn accept_transfer<R: ResolveHeight>(
        &mut self,
        transfer: Transfer,
        resolver: &mut R,
        force: bool,
    ) -> Result<validation::Status, RuntimeError>
    where
        R::Error: 'static,
    {
        self.stock
            .accept_transfer(transfer, resolver, force)
            .map_err(RuntimeError::from)
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        // self.stock
        //     .store(&self.stock_path)
        //     .expect("unable to save stock");
        // let wallets_fd = File::create(&self.wallets_path)
        //     .expect("unable to access wallet file; wallets are not saved");
        // serde_yaml::to_writer(wallets_fd, &self.wallets).expect("unable to save wallets");
    }
}
