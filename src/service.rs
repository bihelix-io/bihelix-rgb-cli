use std::str::FromStr;

/*
 * @Author: jack cymqqqq@gmail.com
 * @Date: 2025-02-25 15:14:34
 * @LastEditors: jack cymqqqq@gmail.com
 * @LastEditTime: 2025-02-25 15:17:13
 * @FilePath: /bihelix-rgb-cli/src/service.rs
 * @Description: 这是默认设置,请设置`customMade`, 打开koroFileHeader查看配置 进行设置: https://github.com/OBKoro1/koro1FileHeader/wiki/%E9%85%8D%E7%BD%AE
 */
use super::*;

pub async fn issue_rgb20(
    ticker: &str,
    name: &str,
    precision: u8,
    issued_supply: u64,
    allocate_outpoint: &str,
    electrum_url: &str,
) -> Result<String, String> {
    let mut wallet = load_wallet();
    let identity = Identity::default().to_string();
    let close_method = bp::dbc::Method::OpretFirst;
    let outpoint = Outpoint::from_str(allocate_outpoint)
        .map_err(|err| format!("parse outpoint failed: {:?}", err))?;
    let issue_precision = Precision::try_from(precision)
        .map_err(|err| format!("parse precision failed: {:?}", err))?;
    let contract_id = wallet
        .issue_rgb20(
            &identity,
            ticker,
            name,
            issue_precision,
            issued_supply,
            close_method,
            outpoint,
            electrum_url,
        )
        .await?;

    Ok(contract_id.to_string())
}
