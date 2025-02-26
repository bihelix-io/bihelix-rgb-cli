use super::*;
pub struct Wallet {
    pub wallet: Runtime,
}

impl Wallet {
    pub async fn import_contract(
        &mut self,
        contract: &ValidContract,
        electrum_url: &str,
    ) -> Result<(), String> {
        self.wallet
            .stock_mut()
            .import_contract(contract.clone(), get_resolver(electrum_url))?;
        self.init_wallet();
        Ok(())
    }

    pub fn init_wallet(&mut self) {
        self.wallet.init_wallet_db().unwrap()
    }

    pub async fn issue_rgb20(
        &mut self,
        identity: &str,
        ticker: &str,
        name: &str,
        precision: Precision,
        issued_supply: u64,
        close_method: CloseMethod,
        allocate_outpoint: Outpoint,
        electrum_url: &str,
    ) -> Result<ContractId, String> {
        let mut kit = Kit::default();
        let _ = kit.ifaces.push(Rgb20::iface(&Rgb20::FIXED));
        let _ = kit.iimpls.push(NonInflatableAsset::issue_impl());
        let _ = kit.schemata.push(NonInflatableAsset::schema());
        let _ = kit
            .scripts
            .extend(NonInflatableAsset::scripts().into_values());
        kit.types = NonInflatableAsset::types();
        let valid_kit = kit.validate().map_err(|(s, _)| s.to_string())?;
        self.wallet.stock_mut().import_kit(valid_kit)?;
        self.init_wallet();
        let contract = Rgb20Wrapper::<MemContract>::testnet::<NonInflatableAsset>(
            identity, ticker, name, None, precision,
        )
        .map_err(|err| format!("invalid contract data: {:?}", err))?
        .allocate(close_method, allocate_outpoint, issued_supply)
        .map_err(|err| format!("invalid allocations: {:?}", err))?
        .issue_contract()
        .map_err(|err| format!("issue contract failed: {:?}", err))?;
        self.import_contract(&contract, electrum_url).await?;

        Ok(contract.contract_id())
    }
}
