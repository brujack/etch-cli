# Installation

There are several ways to install etch-cli on a system: **cargo**, **provided binaries**, or **building from source**.

## Cargo installation

If your system has the Rust programming language tools such as cargo installed, cargo can be used to fetch the source and build a binary on your machine, placing it in location accessible by your path environment variable. First, ensure that the Rust programming language tool is installed by running the following:

```shell
cargo --version
```

You should see the version of cargo installed on your system printed out. If you get an error or nothing, please get the Rust tooling from the [Rust Website](https://www.rust-lang.org/tools/install) if you want to build from source.

Once you have cargo and rustc on your system, you can fetch the sources and build with the following command:

```shell
cargo install etch-cli
```

## Precompiled binaries

Pre-compiled binaries are included on our [GitHub repository](https://github.com/brujack/etch-cli) under our [releases](https://github.com/brujack/etch-cli/releases/).

## Building from source

Building from source should be a straight forward task for anyone familiar with the Rust toolchain. It is recommended that you read through the [cargo book](https://doc.rust-lang.org/cargo/) and get familiar with it. Once you are, building is a matter of simply cloning our repository and compiling it. However, it is important to note that you may need to ensure you have the development libraries for openssl installed on your system. Check with your operating system and package manager what these packages are as they can often vary in naming between different systems.

```shell
git clone https://github.com/brujack/etch-cli.git
cd etch-cli
cargo build --release
# binary at target/release/etch
```
