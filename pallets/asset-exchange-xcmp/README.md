# pallet-asset-exchange-xcmp

## Purpose

This pallet aims to provide functionalities for sending and handling transfers of parachain assets and main network currency between the exchange pallet's parachain and other parachains/relay chain.

## Dependencies

### Configs

This pallet depends on `pallet_asset_exchange::Config` to perform the deposit and withdrawal of assets.

[pallet_asset_exchange](https://github.com/mattsse/substrate-exchange-xcmp/tree/master/pallets/asset-exchange/)

## Installation

### Runtime `Cargo.toml`

To add this pallet to your runtime, simply include the following lines to your runtime's `Cargo.toml` file:

```TOML
pallet-asset-exchange = { git = "https://github.com/mattsse/substrate-exchange-xcmp", default-features = false }
pallet-asset-exchange-xcmp = { git = "https://github.com/mattsse/substrate-exchange-xcmp", default-features = false }
```

and update your runtime's `std` feature to include this pallet:

```TOML
std = [
    # --snip--
    'pallet-asset-exchange/std',
    'pallet-asset-exchange-xcmp/std',
]
```

### Runtime `lib.rs`

You should implement related configs like so, please check up [lib.rs](https://github.com/mattsse/substrate-exchange-xcmp/blob/master/runtime/src/lib.rs#L336) for details:

To include it in your `construct_runtime!` macro:

```rust
AssetExchange: pallet_asset_exchange::{Module, Call, Config<T>, Storage, Event<T>},
AssetExchangeXcmp: pallet_asset_exchange_xcmp::{Module, Call, Storage, Event<T>}
```
