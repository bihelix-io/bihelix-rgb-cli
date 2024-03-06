set -e

CLOSING_METHOD="opret1st"
DERIVE_PATH="m/86'/1'/0'/9"
DESC_TYPE="wpkh"
ELECTRUM="blockstream.info:143"
CONSIGNMENT="consignment.rgb"
PSBT="tx.psbt"
IFACE="RGB20"
contract_id=""
program() {
     ../target/release/bihelix-rgb-cli $@
}
rgb() {
    program -n testnet rgb -d data -s "$ELECTRUM" $@
}


rgb state "$contract_id" "$IFACE"
