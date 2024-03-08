set -e

CLOSING_METHOD="opret1st"
DERIVE_PATH="m/86'/1'/0'/9"
DESC_TYPE="wpkh"
ELECTRUM="blockstream.info:143"
CONSIGNMENT="consignment.rgb"
PSBT="tx.psbt"
IFACE="RGB20"

program() {
    target/release/bihelix-rgb-cli $@
}
rgb0() {
    program -n testnet rgb -d data0 -s "$ELECTRUM" $@
}
mkdir data{0,core,index}

xprv_0=$(program --network testnet key generate | jq -r '.xprv')
output=$(program --network testnet key derive -p "$DERIVE_PATH" -x "$xprv_0")
xprv_der_0=$(echo $output | jq -r '.xprv')
xpub_der_0=$(echo $output | jq -r '.xpub')
echo $xprv_der_0
echo $xpub_der_0
addr_issue=$(program --network testnet wallet -w issuer -d "$DESC_TYPE($xpub_der_0)" get-new-address | jq -r '.address')
echo $addr_issue