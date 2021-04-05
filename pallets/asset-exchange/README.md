# pallet-asset-exchange

## Purpose

This pallet provides the core functionalities for depositing, withdrawing assets and assets as liquidity, as well as (multi) swapping assets.

## Installation

### Runtime `Cargo.toml`

To add this pallet to your runtime, simply include the following line to your runtime's `Cargo.toml` file:

```TOML
pallet-asset-exchange = { git = "https://github.com/mattsse/substrate-exchange-xcmp", default-features = false }
```

and update your runtime's `std` feature to include this pallet:

```TOML
std = [
    # --snip--
    'pallet-asset-exchange/std',
]
```

### Runtime `lib.rs`

You should implement related traits like so, please check up [lib.rs](https://github.com/mattsse/substrate-exchange-xcmp/blob/master/pallets/asset-exchange/src/mock.rs#L67) for a basic example:

```rust
impl pallet_asset_exchange::Config for YourRuntime {
    type Event = Event;
    type ExchangeAdmin = frame_system::EnsureRoot<Self::AccountId>;
    type PoolId = u64;
    type MaxPoolLimit = MaxPoolLimit;
    type AssetId = u64;
    type Currency = pallet_balances::Module<Test>;
    type FeeDivisor = FeeDivisor;
    type ExchangeFee = ExchangeFee;
    type ReferralFee = ReferralFee;
    type MinNativeAssetAmount = MinNativeAssetAmount;
    type MinParachainAssetAmount = MinParachainAssetAmount;
    type InitSharesSupply = InitialSharesSupply;
}
```

and include it in your `construct_runtime!` macro:

```rust
AssetExchange: pallet_asset_exchange::{Module, Config<T>, Call, Storage, Event<T>},
```

### Genesis Configuration example
```rust
 pallet_asset_exchange: AssetExchangeConfig {
            allowed_assets: vec![b"DOT".to_vec()],
            exchange_account: Some(get_account_id_from_seed::<sr25519::Public>("Alice")),
        }
```
## Reference Docs

You can view the reference docs for this pallet by running:

```
cargo doc --open
```