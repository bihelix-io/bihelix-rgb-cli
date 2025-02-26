use bpstd::Network;
use rgb::resolvers::AnyResolver;

/*
 * @Author: jack cymqqqq@gmail.com
 * @Date: 2025-02-25 14:13:37
 * @LastEditors: jack cymqqqq@gmail.com
 * @LastEditTime: 2025-02-26 09:07:11
 * @FilePath: /bihelix-rgb-provider/src/utils.rs
 * @Description: 这是默认设置,请设置`customMade`, 打开koroFileHeader查看配置 进行设置: https://github.com/OBKoro1/koro1FileHeader/wiki/%E9%85%8D%E7%BD%AE
 */
use super::*;

pub fn get_resolver(url: &str) -> AnyResolver {
    let resolver = AnyResolver::electrum_blocking(url, None).unwrap();
    resolver.check(Network::Testnet4).unwrap();
    resolver
}

pub fn load_wallet() -> Wallet {
    let rgb_wallet = load_rgb_data_fromdb("STOCK")
        .map_err(|err| format!("load data failed {:?}", err))
        .unwrap();
    let store = Runtime::attach("STOCK", rgb_wallet);
    Wallet { wallet: store }
}

pub async fn generate_wallet() {
    let stock = Stock::in_memory();
    let store = Runtime::attach("STOCK", stock);

    let mut wallet = Wallet { wallet: store };
    wallet.init_wallet();
}

pub fn load_rgb_data_fromdb(db_name: &str) -> Result<Stock, String> {
    let db = RuntimeStore::new(db_name).unwrap();

    // 加载 stash, index, 和 state 数据
    let stash_data = db.get("STASH".as_bytes()).unwrap().unwrap();
    let index_data = db.get("INDEX".as_bytes()).unwrap().unwrap();
    let state_data = db.get("STATE".as_bytes()).unwrap().unwrap();

    Stock::<MemStash, MemState, MemIndex>::load_stock_bytes(&stash_data, &index_data, &state_data)
        .map_err(|err| {
            format!(
                "Deserialization failed for stash: {:?}, index: {:?}, state: {:?}. Error: {:?}",
                stash_data.len(),
                index_data.len(),
                state_data.len(),
                err
            )
        })
}
