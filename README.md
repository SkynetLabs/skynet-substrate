# Skynet Substrate

## Overview

This is the official Skynet SDK for Substrate development in Rust!

It contains functions for uploading and downloading data as well as for interacting with the Skynet registry.

## Examples

- [Skynet Offchain Worker Example Pallet](https://github.com/SkynetLabs/skynet-substrate-offchain-worker-node/tree/skynet-substrate/frame/examples/offchain-worker)

## Developing

### Testing

Tests can be run by executing

```
cargo test
```

Code coverage can be profiled by installing and running `cargo-tarpaulin` (Linux-only):

```
cargo install cargo-tarpaulin
cargo tarpaulin
```

If you're not developing on a Linux system, you can just raise a PR on our repo
and our CI will profile the coverage.

## Docs

See the docs [here](https://skynetlabs.github.io/skynet-substrate/skynet_substrate/).

The docs were built with:

```sh
cargo doc --no-deps
rm -rf ./docs
echo '<meta http-equiv="refresh" content="0; url=skynet_substrate/">' > target/doc/index.html
cp -r target/doc ./docs
```
