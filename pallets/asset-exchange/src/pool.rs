use frame_support::pallet_prelude::Member;
use frame_support::pallet_prelude::*;
use frame_support::Parameter;
use sp_std::{collections::btree_map::BTreeMap, fmt::Debug, prelude::*};

pub trait Pool: Parameter + Member {}

#[derive(Clone, PartialEq, Eq, Encode, Decode, Default, Debug)]
pub struct SimplePool<AccountId: Ord, AssetId: Ord, Balance> {
    /// List of assets in the pool.
    pub asset_ids: Vec<AssetId>,
    /// Assets of the pool by liquidity providers.
    pub assets: BTreeMap<AccountId, Balance>,
    /// Fee charged for swap (gets divided by FEE_DIVISOR).
    pub total_fee: Balance,
    /// Portion of the fee going to exchange.
    pub exchange_fee: Balance,
    /// Portion of the fee going to referral.
    pub referral_fee: Balance,
    /// Total number of assets.
    pub assets_total_supply: Balance,
}
