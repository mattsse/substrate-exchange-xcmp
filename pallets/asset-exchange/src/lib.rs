//! # Asset Exchange Implementation
//!
//! This pallet exposes an exchange for managing multiple that allow for atomic trades between them
//!
//! - [`pallet_asset_exchange::Trait`](./trait.Trait.html)
//! - [`Calls`](./enum.Call.html)
//! - [`Errors`](./enum.Error.html)
//! - [`Events`](./enum.RawEvent.html)
//!
//! ## Overview
//!
//!
//! ## Dispatchable Functions
//!
//!
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod pool;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use crate::pool::Pool;
    use frame_support::traits::Currency;
    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    use sp_std::{collections::btree_map::BTreeMap, fmt::Debug, prelude::*};

    /// Represents asset balances
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Information about account asset deposits
    #[derive(Encode, Decode, Clone, Default, Debug)]
    pub struct AccountDeposit<AssetId: Ord, Balance> {
        /// balance of native currency sent to the exchange
        pub amount: Balance,
        /// Amounts of various assets in this account.
        pub assets: BTreeMap<AssetId, Balance>,
    }

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The dispatch origin that owns this exchange and is able to (un)register new pools.
        type ExchangeAdmin: EnsureOrigin<Self::Origin>;

        /// The type pools this exchange manages
        type Pool: Pool;

        /// Representation of Assets
        type AssetId: Member + Parameter + Ord;

        /// Main currency
        type Currency: Currency<Self::AccountId>;

        /// Exchange fee, that goes to exchange itself
        type ExchangeFee: Get<BalanceOf<Self>>;

        /// Referral fee, that goes to referrer in the call
        type ReferralFee: Get<BalanceOf<Self>>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // The pallet's runtime storage items.
    // https://substrate.dev/docs/en/knowledgebase/runtime/storage

    #[pallet::storage]
    #[pallet::getter(fn deposited_amounts)]
    /// amount of deposited assets for each account
    pub type DepositedAmounts<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AccountDeposit<T::AssetId, BalanceOf<T>>>;

    #[pallet::storage]
    #[pallet::getter(fn pools)]
    /// a list of all the available pools
    pub type Pools<T: Config> = StorageValue<_, Vec<T::Pool>>;

    #[pallet::storage]
    #[pallet::getter(fn allowed_assets)]
    /// set of assets allowed by the owner of the exchange
    pub type AllowedAssets<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetId, ()>;

    // The pallet's runtime storage items.
    // https://substrate.dev/docs/en/knowledgebase/runtime/storage
    #[pallet::storage]
    #[pallet::getter(fn something)]
    // Learn more about declaring storage items:
    // https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
    pub type Something<T> = StorageValue<_, u32>;

    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event documentation should end with an array that provides descriptive names for event
        /// parameters. [something, who]
        SomethingStored(u32, T::AccountId),

        /// Amounts of assets deposited into a pool
        AddedLiquidity,

        /// Amounts of assets withdrawn from a pool
        RemovedLiquidity,

        /// Added a new pool
        PoolAdded,

        /// Removed an existing pool
        PoolRemoved,

        /// Swapped a set of assets
        Swapped,

        /// Registered an asset in the user's account deposit
        RegisteredAsset,

        /// Unregistered an asset from a user's account
        UnRegisteredAsset,
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Error names should be descriptive.
        NoneValue,
        /// Errors should have helpful documentation associated with them.
        StorageOverflow,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// An example dispatchable that takes a singles value as a parameter, writes the value to
        /// storage and emits an event. This function must be dispatched by a signed extrinsic.
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://substrate.dev/docs/en/knowledgebase/runtime/origin
            let who = ensure_signed(origin)?;

            // Update storage.
            <Something<T>>::put(something);

            // Emit an event.
            Self::deposit_event(Event::SomethingStored(something, who));
            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// An example dispatchable that may throw a custom error.
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn cause_error(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let _who = ensure_signed(origin)?;

            // Read a value from storage.
            match <Something<T>>::get() {
                // Return an error if the value has not been set.
                None => Err(Error::<T>::NoneValue)?,
                Some(old) => {
                    // Increment the value read from storage; will error in the event of overflow.
                    let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
                    // Update the value in storage with the incremented result.
                    <Something<T>>::put(new);
                    Ok(().into())
                }
            }
        }
    }
}
