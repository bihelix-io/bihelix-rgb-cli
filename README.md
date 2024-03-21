
## Bihelix RGB CLI
The rgb command-line tool, which combined with the Bitcoin Development Kit and the rgb 0.11 libraries, developed by the Behelix Team is used to import, export, issue, and query your off-chain RGB20(currently support RGB20) assets.
Notes: In the current stage, the user can only import, and query their rgb20 asset by this cli tool, can not transfer the rgb asset(bihelix-cli don't include rgb single transfer).
## Usage

### Install the program
To git the repo and then compile it.
```shell
git clone https://github.com/BiHelix-Labs/bihelix-rgb-cli.git
cd bihelix-rgb-cli
cargo build --release
```

### Query the RGB20 asset
```shell
# based on your stock.dat, contrac id, you can run this shell to query your asset in the local
bihelix-rgb-cli -n network rgb -d $rgb_stash state $contract_id RGB20 --address $btc_address
```

