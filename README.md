
## Bihelix RGB CLI
The rgb command-line tool, which combined with the Bitcoin Development Kit and the rgb 0.11 libraries, developed by the Behelix Team is used to import, export, issue, and query your off-chain RGB20(currently support RGB20) assets.

## Usage

### Install the program
1.first step
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
"[2b3050bc/86'/1'/0'/9]tprv8hy89tKrseaHXP5mWnWao7Q9nHEge994uuzd9PUGLBdwhyKmCrvhENVY6ETQUE1zJh8EoXj9sxU6AGSGEWDG3EAoEYx2NCyQqoJByHiKbpt/*
[2b3050bc/86'/1'/0'/9]""tpubDEfAJJN722FxQr7ZQSBBCX4GMJkcoUKyVDbQRuWZkTSLYTaXqFkHQs7QGNut824tftQaPavf3D4XJFLXZwcUZ2fyhiG4pmRCufsGwACps8g/*"
```
