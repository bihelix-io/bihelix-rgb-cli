
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

### Generate your pub/prv key
```shell

# generate your first bdk wallet descriptor
bash demo/run_generate_address.sh
# then it will prints out, so it's your public key and private key, you can use it to create transaction and sign transaction:
"[2b3050bc/86'/1'/0'/9]tprv8hy89tKrseaHXP5mWnWao7Q9nHEge994uuzd9PUGLBdwhyKmCrvhENVY6ETQUE1zJh8EoXj9sxU6AGSGEWDG3EAoEYx2NCyQqoJByHiKbpt/*"
"[2b3050bc/86'/1'/0'/9]tpubDEfAJJN722FxQr7ZQSBBCX4GMJkcoUKyVDbQRuWZkTSLYTaXqFkHQs7QGNut824tftQaPavf3D4XJFLXZwcUZ2fyhiG4pmRCufsGwACps8g/*"
```

### Query the RGB20 asset
```shell

# based on your stock.dat, contrac id, and rgb consigment file, you can run this shell to query your asset in the local
bash demo/query_asset.sh
```

