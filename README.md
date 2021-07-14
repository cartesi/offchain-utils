# Offchain utils v2

## Running the project

### Pre requirements

* `rust` - https://rustup.rs/
* `solc` version 0.7.0 - https://github.com/ethereum/solidity/releases/tag/v0.7.0
* `geth` - https://geth.ethereum.org/docs/install-and-build/installing-geth

#### Linux with apt

#### Dependencies

```
$ apt install pkg-config build-essential curl libssl-dev
```

##### Rust, using rustup

```
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ source $HOME/.cargo/env
```

Check installation with:

```
$ rustup show
Default host: x86_64-unknown-linux-gnu
rustup home:  /root/.rustup

stable-x86_64-unknown-linux-gnu (default)
rustc 1.51.0 (2fd73fabe 2021-03-23)
```

##### Solidity with `solc` version 0.7.0

```
$ apt install cmake z3 cvc4 libboost-all-dev
$ git clone git@github.com:ethereum/solidity.git
$ cd solidity/
$ git checkout 9e61f92bd4d19b430cb8cb26f1c7cf79f1dff380
$ ./scripts/build.sh
```

Check installation with:

```
$ solc --version
solc, the solidity compiler commandline interface
Version: 0.7.0+commit.9e61f92b.Linux.g++
```

##### Geth

```
$ apt install software-properties-common
$ add-apt-repository -y ppa:ethereum/ethereum
$ apt update
$ apt install ethereum
```

Check installation with:

```
$ geth version
Geth
Version: 1.10.2-stable
Git Commit: 97d11b0187b4695ccf44e3b71b54155fe405a36f
Architecture: amd64
Go Version: go1.16
Operating System: linux
GOPATH=
GOROOT=go
```

#### MacOS with homebrew

##### Rust, using rustup

```
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Check installation with:

```
$ rustup show
Default host: x86_64-apple-darwin
rustup home:  /Users/username/.rustup

stable-x86_64-apple-darwin (default)
rustc 1.51.0 (2fd73fabe 2021-03-23)
```

##### Solidity with `solc` version 0.7.0

```
$ git clone https://github.com/ethereum/homebrew-ethereum.git 
$ cd homebrew-ethereum
$ git checkout f74ccbe175e5268ce68565c416504da55da2893a
$ brew unlink solidity
$ brew install solidity.rb
```

Check installation with:

```
$ solc --version
solc, the solidity compiler commandline interface
Version: 0.7.0+commit.9e61f92b.Darwin.appleclang
```

##### Geth

```
$ brew tap ethereum/ethereum
$ brew install ethereum
```

Check installation with:

```
$ geth version
Geth
Version: 1.10.2-stable
Architecture: amd64
Go Version: go1.16.3
Operating System: darwin
GOPATH=
GOROOT=go
```

### Check build

Just run `cargo build`:

```
$ cargo build
   Compiling state_fold v0.1.0 (/somefolder/dispatcher-v2/state_fold)
   Compiling dispatcher v0.1.0 (/somefolder/dispatcher-v2/dispatcher)
    Finished dev [unoptimized + debuginfo] target(s) in 5.62s
```

### Run tests

Just run `cargo test`. All tests should pass. 

```
$ cargo test
   Compiling state_fold v0.1.0 (/somefolder/dispatcher-v2/state_fold)
   Compiling dispatcher v0.1.0 (/somefolder/dispatcher-v2/dispatcher)
    Finished test [unoptimized + debuginfo] target(s) in 17.09s
     Running target/debug/deps/backoff-463e2a606c9b79b0

(... a lot of test output ...)
```
