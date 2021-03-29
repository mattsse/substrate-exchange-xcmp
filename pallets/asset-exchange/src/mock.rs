use frame_support::parameter_types;
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use crate as pallet_asset_exchange;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
        AssetExchangeModule: pallet_asset_exchange::{Module, Call, Storage, Event<T>}
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

parameter_types! {
    pub const InitialSharesSupply: u128 = 1_000_000_000_000_000_000_000_000;
    pub const FeeDivisor:u128 = 10_000;
    pub const MaxPoolLimit:u64 = 100_000_000;
    pub const ExchangeFee:u128 = 25;
    pub const ReferralFee:u128 = 0;
    pub const MinNativeAssetAmount:u128 = 0;
    pub const MinParachainAssetAmount:u128 = 0;
}

impl pallet_asset_exchange::Config for Test {
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

parameter_types! {
    pub const ExistentialDeposit: u128 = 500;
}

/// Balance of an account.
impl pallet_balances::Config for Test {
    type Balance = u128;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Module<Test>;
    type MaxLocks = ();
    type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 100_000),
            (2, 100_000),
            (3, 100_000),
            (4, 100_000),
            (5, 100_000),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    pallet_asset_exchange::GenesisConfig::<Test> {
        allowed_assets: vec![1, 2, 3, 4, 5],
        exchange_account: Some(1),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
