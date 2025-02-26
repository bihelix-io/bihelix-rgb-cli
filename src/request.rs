/*
 * @Author: jack cymqqqq@gmail.com
 * @Date: 2025-02-25 15:46:21
 * @LastEditors: jack cymqqqq@gmail.com
 * @LastEditTime: 2025-02-25 15:52:16
 * @FilePath: /bihelix-rgb-cli/src/request.rs
 * @Description: 这是默认设置,请设置`customMade`, 打开koroFileHeader查看配置 进行设置: https://github.com/OBKoro1/koro1FileHeader/wiki/%E9%85%8D%E7%BD%AE
 */
use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueRgb20Request {
    pub ticker: String,
    pub name: String,
    pub precision: u8,
    pub issued_supply: u64,
    pub allocate_outpoint: String,
    pub electrum_url: String,
}
