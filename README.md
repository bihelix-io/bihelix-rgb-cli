
# BiHelix RGB CLI
The rgb command-line tool which combined with the Bitcoin Development Kit and the rgb 0.11 libraries, developed by the Behelix Team is used to import, export, issue, and query your off-chain RGB20(currently support RGB20) assets.
Notes: In the current stage, the user can only import, and query their rgb20 asset by this cli tool, can not transfer the rgb asset(bihelix-cli don't include rgb single transfer).

# Requirements
Before run this repo, please check whether or not your rust version satisfy 1.76, if not, please update it to the 1.76 version(Or the latest stable version).

# Installation
Clone the repo from the github and compile it.
```bash
git clone https://github.com/bihelix-io/bihelix-rgb-cli
cd bihelix-rgb-cli
cargo build --release
cd ./target/release
```

# Usage
## Prepare the data
- Get the RGB20 token **contract id**
- Create **./data/bitcoin** directory if not exists
- Copy **stock.dat** file into ./data/bitcoin directory

## Query the RGB20 asset
Open PowerShell(Windows) or Terminal(MacOS) and types cmd as followed.
```bash
./bihelix-rgb-cli -n bitcoin rgb -d ./data state [contract id] RGB20 --address [your bitcoin address]
```

# Notes
## amplify_num bugs
![alt text](image.png)
If anyone meet this error, please run 
`sudo cargo update amplify_num --precise 0.5.1`
then, run 
`sudo cargo build --release` 
again