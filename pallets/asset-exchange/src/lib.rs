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

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use pallet::*;

mod pool;

#[frame_support::pallet]
pub mod pallet {
    use crate::pool::{BasicPool, Pool, PoolInfo};
    use frame_support::sp_runtime::traits::*;
    use frame_support::traits::Currency;
    #[cfg(feature = "std")]
    use frame_support::traits::GenesisBuild;
    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    use sp_std::{collections::btree_map::BTreeMap, fmt::Debug, prelude::*};

    /// Represents supported assets in the exchange, either native currency or a representation for assets from other parachains
    #[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Asset<AssetId: Default + Debug + Ord + Copy> {
        Native,
        ParachainAsset(AssetId),
    }

    /// Represents asset balances
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Represents an asset and its balance
    #[derive(Encode, Decode, PartialEq, Eq, Clone, Default, Debug)]
    pub struct AssetBalance<AssetId, Balance> {
        /// the asset to deposit
        pub asset: AssetId,
        /// The amount to deposit
        pub amount: Balance,
    }

    impl<AssetId, Balance> AssetBalance<AssetId, Balance> {
        pub fn new(asset: AssetId, amount: Balance) -> Self {
            Self { asset, amount }
        }
    }

    /// Makes sure only unique assets are in a list of assets and their balances
    pub fn ensure_unique_assets<T: Config>(
        assets: Vec<AssetBalance<T::AssetId, BalanceOf<T>>>,
    ) -> Result<BTreeMap<T::AssetId, BalanceOf<T>>, Error<T>> {
        let assets_len = assets.len();
        let asset_set: BTreeMap<_, _> = assets.into_iter().map(|x| (x.asset, x.amount)).collect();
        if asset_set.len() == assets_len {
            Ok(asset_set)
        } else {
            Err(Error::<T>::DuplicateAsset)
        }
    }

    /// Information about the account's asset deposits
    #[derive(Encode, Decode, Clone, Debug)]
    pub struct AccountDeposit<T: Config> {
        /// balance of native currency sent to the exchange
        pub amount: BalanceOf<T>,
        /// Amounts of various assets in this account.
        pub assets: BTreeMap<T::AssetId, BalanceOf<T>>,
    }

    impl<T: Config> Default for AccountDeposit<T> {
        fn default() -> Self {
            Self {
                amount: BalanceOf::<T>::zero(),
                assets: BTreeMap::new(),
            }
        }
    }

    impl<T: Config> AccountDeposit<T> {
        /// Subtract from balance of given asset.
        pub fn sub(
            &mut self,
            asset_id: &T::AssetId,
            amount: BalanceOf<T>,
        ) -> Result<BalanceOf<T>, Error<T>> {
            let value = self
                .assets
                .get_mut(asset_id)
                .ok_or(Error::<T>::AssetNotFound)?;
            let new_balance = value
                .checked_sub(&amount)
                .ok_or(Error::<T>::NotEnoughBalance)?;
            *value = new_balance;
            Ok(new_balance)
        }

        /// Mocks subtracting from the users balance
        fn can_sub(
            &self,
            asset_id: &T::AssetId,
            amount: BalanceOf<T>,
        ) -> Result<BalanceOf<T>, Error<T>> {
            let value = self.assets.get(asset_id).ok_or(Error::<T>::AssetNotFound)?;
            value
                .checked_sub(&amount)
                .ok_or(Error::<T>::NotEnoughBalance)
        }

        /// Adds amount to the balance of given asset.
        pub fn add(&mut self, asset_id: &T::AssetId, amount: BalanceOf<T>) -> Result<(), Error<T>> {
            let value = self
                .assets
                .get_mut(asset_id)
                .ok_or(Error::<T>::AssetNotFound)?;
            let new_balance = value
                .checked_add(&amount)
                .ok_or(Error::<T>::StorageOverflow)?;
            *value = new_balance;

            Ok(())
        }
    }

    /// Single asset swap action, represents some `amount` of `asset_in` is being exchanged to `asset_out`
    #[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
    pub struct AssetSwap<AssetId, Balance, PoolId> {
        /// Pool which should be used for swapping.
        pub pool_id: PoolId,
        /// Asset to swap from.
        pub asset_in: AssetId,
        /// Amount to exchange.
        ///
        /// If amount_in is None, it will take amount_out from previous step.
        /// Will fail if amount_in is None on the first step.
        pub amount_in: Option<Balance>,
        /// Asset to swap into.
        pub asset_out: AssetId,
        /// Required minimum amount of asset_out.
        pub min_amount_out: Balance,
    }

    /// Information about a completed `AssetSwap`
    #[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
    pub struct AssetSwapInfo<AssetId, Balance, PoolId> {
        /// The pool the swap occurred
        pub pool_id: PoolId,
        /// Asset to swap from.
        pub asset_in: AssetId,
        /// Amount to exchange.
        ///
        /// The amount exchanged
        pub amount_in: Balance,
        /// Asset to swap into.
        pub asset_out: AssetId,
        /// Swapped amount of asset_out.
        pub amount_out: Balance,
    }

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The dispatch origin that owns this exchange and is able to (un)register new pools.
        type ExchangeAdmin: EnsureOrigin<Self::Origin>;

        /// Identifier for a pool
        type PoolId: Member + Parameter + Copy + AtLeast32BitUnsigned + From<u64>;

        type MaxPoolLimit: Get<u64>;

        /// Representation of Assets
        type AssetId: Member + Parameter + Ord + MaybeSerializeDeserialize;

        /// Main currency
        type Currency: Currency<Self::AccountId>;

        /// The fee divisor
        type FeeDivisor: Get<BalanceOf<Self>>;

        /// Exchange fee, that goes to exchange itself
        type ExchangeFee: Get<BalanceOf<Self>>;

        /// Referral fee, that goes to referrer in the call
        type ReferralFee: Get<BalanceOf<Self>>;

        /// Minimum amount of native currency to execute add/remove liquidity operations
        type MinNativeAssetAmount: Get<BalanceOf<Self>>;

        /// Minimum parachain asset amount execute add/remove liquidity operations.
        type MinParachainAssetAmount: Get<BalanceOf<Self>>;

        /// Initial shares supply on deposit of liquidity.
        type InitSharesSupply: Get<BalanceOf<Self>>;
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
        StorageMap<_, Blake2_128Concat, T::AccountId, AccountDeposit<T>>;

    #[pallet::storage]
    #[pallet::getter(fn exchange_account)]
    /// The account that should get payed for every trade
    pub type ExchangeAccount<T: Config> = StorageValue<_, T::AccountId>;

    #[pallet::storage]
    #[pallet::getter(fn pools)]
    /// A list of all the available pools
    pub type Pools<T: Config> = StorageMap<_, Blake2_128Concat, T::PoolId, Pool<T>>;

    #[pallet::storage]
    #[pallet::getter(fn pool_count)]
    /// The total number of pools currently stored in the map.
    ///
    /// Because the map does not store its size, we store it separately
    pub type PoolCount<T: Config> = StorageValue<_, u64>;

    #[pallet::storage]
    #[pallet::getter(fn pool_id_counter)]
    /// Counter increment for issuing unique IDs
    ///
    /// 2^64 should be enough, but you never know...
    pub type PoolIdCounter<T: Config> = StorageValue<_, u64>;

    #[pallet::storage]
    #[pallet::getter(fn allowed_assets)]
    /// set of assets allowed by the owner of the exchange
    pub type AllowedAssets<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetId, ()>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub allowed_assets: Vec<T::AssetId>,
        pub exchange_account: Option<T::AccountId>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                allowed_assets: Default::default(),
                exchange_account: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            if let Some(acc) = self.exchange_account.clone() {
                ExchangeAccount::<T>::put(acc);
            }
            for asset in self.allowed_assets.iter() {
                AllowedAssets::<T>::insert(asset.clone(), ());
            }
        }
    }

    #[cfg(feature = "std")]
    impl<T: Config> GenesisConfig<T> {
        /// Direct implementation of `GenesisBuild::build_storage`.
        ///
        /// Kept in order not to break dependency.
        pub fn build_storage(&self) -> Result<frame_support::sp_runtime::Storage, String> {
            <Self as GenesisBuild<T>>::build_storage(self)
        }

        /// Direct implementation of `GenesisBuild::assimilate_storage`.
        ///
        /// Kept in order not to break dependency.
        pub fn assimilate_storage(
            &self,
            storage: &mut frame_support::sp_runtime::Storage,
        ) -> Result<(), String> {
            <Self as GenesisBuild<T>>::assimilate_storage(self, storage)
        }
    }

    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId", T::PoolId = "Pool", T::AssetId = "Asset")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Amounts of assets deposited into a pool
        AddedLiquidity(T::AccountId, Vec<AssetBalance<T::AssetId, BalanceOf<T>>>),

        /// Amounts of assets withdrawn from a pool
        RemovedLiquidity,

        /// Added a new pool. [creator, pool identifier, assets in pool]
        PoolAdded(T::AccountId, T::PoolId, Vec<T::AssetId>),

        /// Swapped a set of assets [account, asset swaps]
        Swapped(
            T::AccountId,
            Vec<AssetSwapInfo<T::AssetId, BalanceOf<T>, T::PoolId>>,
        ),

        /// Registered an asset in the user's account deposit [owner, asset identifiers]
        RegisteredAssets(T::AccountId, Vec<T::AssetId>),

        /// Unregistered an asset from a user's account [owner, asset identifiers]
        UnRegisteredAssets(T::AccountId, Vec<T::AssetId>),

        /// Withdrawn user deposits [owner, asset, balance]
        WithDrawn(T::AccountId, T::AssetId, BalanceOf<T>),

        /// Deposited into user's account [owner, asset, balance]
        Deposited(T::AccountId, T::AssetId, BalanceOf<T>),

        /// Deposited into user's account [previous account, new account]
        SetExchangeAccount(Option<T::AccountId>, T::AccountId),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Errors should have helpful documentation associated with them.
        StorageOverflow,
        /// Maximum amount of pools issued
        PoolLimitReached,
        /// Thrown when the user doesn't have enough balance in his account
        NotEnoughBalance,
        /// Thrown when the user is not registered yet
        AccountNotFound,
        /// Thrown when the asset is not found
        AssetNotFound,
        /// Thrown when the asset is not allowed
        AssetNotAllowed,
        /// Thrown when an asset is already registered
        DuplicateAsset,
        /// Thrown when an asset can't be unregistered because balance is non zero
        NonZeroBalance,
        /// Thrown when no matching pool was found
        PoolNotFound,
        /// Thrown when a liquidity deposit doesn't correspond with the assets registered in a pool
        InvalidLiquidityDeposit,
        /// Thrown when a new pool can not be initiated because the fee is too large
        PoolFeeTooLarge,
        /// Thrown when a pool can not be initiated because too few asset types were supplied
        InvalidPoolAssets,
        /// Thrown only when the `T::PoolId` type was configured to produce duplicate values
        /// for different counter inputs `T::PollId : From<u64>`
        InvalidPoolId,
        /// Thrown when the invariant after a swap is larger than before
        // TODO can this even happen?
        InvalidCurveInvariant,
        /// Thrown when no initial swap amount in was set in a series of asset swap actions
        MissingSwapAmountIn,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Sets the exchange's own that should receive fees for trades
        #[pallet::weight(10_000)]
        pub fn set_exchange_account(
            origin: OriginFor<T>,
            new_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            T::ExchangeAdmin::ensure_origin(origin)?;
            let old = ExchangeAccount::<T>::mutate(|old| old.replace(new_account.clone()));
            Self::deposit_event(Event::SetExchangeAccount(old, new_account));

            Ok(().into())
        }

        /// Add liquidity from already deposited amounts to a pool
        #[pallet::weight(10_000)]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            pool_id: T::PoolId,
            asset_deposits: Vec<AssetBalance<T::AssetId, BalanceOf<T>>>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            let mut asset_deposits = ensure_unique_assets::<T>(asset_deposits)?;

            let mut deposit =
                DepositedAmounts::<T>::try_get(&who).map_err(|_| Error::<T>::AccountNotFound)?;

            for (asset, balance) in &asset_deposits {
                deposit.can_sub(asset, *balance)?;
            }

            Pools::<T>::try_mutate(&pool_id, |maybe_pool| match maybe_pool {
                None => Err(Error::<T>::PoolNotFound),
                Some(pool) => {
                    // NOTE: this should fail without writing to storage
                    pool.add_liquidity(&who, &mut asset_deposits)?;
                    Ok(())
                }
            })?;

            for (asset, balance) in &asset_deposits {
                deposit
                    .sub(asset, *balance)
                    .expect("We already checked that user has enough balance; qed");
            }

            DepositedAmounts::<T>::insert(who.clone(), deposit);

            Self::deposit_event(Event::AddedLiquidity(
                who,
                asset_deposits
                    .into_iter()
                    .map(|(id, balance)| AssetBalance::new(id, balance))
                    .collect(),
            ));

            Ok(().into())
        }

        /// Registers given assets in the user's account deposit.
        ///
        /// Fails if no matching account found for the user or an asset was already registered.
        #[pallet::weight(10_000)]
        pub fn register_assets(
            origin: OriginFor<T>,
            asset_ids: Vec<T::AssetId>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            DepositedAmounts::<T>::try_mutate(&who, |maybe_deposit| match maybe_deposit {
                None => Err(Error::<T>::AccountNotFound),
                Some(deposit) => {
                    for id in &asset_ids {
                        // fail if asset already registered
                        ensure!(!deposit.assets.contains_key(id), Error::<T>::DuplicateAsset);
                        // TODO validate that sufficient amount is in the account
                    }
                    // insert empty balance for each asset
                    deposit.assets.extend(
                        asset_ids
                            .iter()
                            .cloned()
                            .map(|id| (id, BalanceOf::<T>::zero())),
                    );
                    Ok(())
                }
            })?;

            Self::deposit_event(Event::RegisteredAssets(who, asset_ids));

            Ok(().into())
        }

        /// Unregisters given assets in the user's account deposit.
        ///
        /// Fails if the balance of any given asset is non zero.
        #[pallet::weight(10_000)]
        pub fn unregister_assets(
            origin: OriginFor<T>,
            asset_ids: Vec<T::AssetId>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            DepositedAmounts::<T>::try_mutate(&who, |maybe_deposit| match maybe_deposit {
                None => Err(Error::<T>::AccountNotFound),
                Some(deposit) => {
                    for id in &asset_ids {
                        // fail if balance is non 0
                        if let Some(balance) = deposit.assets.get(id) {
                            ensure!(balance.is_zero(), Error::<T>::NonZeroBalance);
                        }
                    }
                    // remove all asset ids
                    for id in &asset_ids {
                        deposit.assets.remove(id);
                    }
                    Ok(())
                }
            })?;

            Self::deposit_event(Event::UnRegisteredAssets(who, asset_ids));

            Ok(().into())
        }

        /// Withdraws given asset from the deposits of the given user.
        #[pallet::weight(10_000)]
        pub fn withdraw(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            DepositedAmounts::<T>::try_mutate(&who, |maybe_deposit| match maybe_deposit {
                None => Err(Error::<T>::AccountNotFound),
                Some(deposit) => deposit.sub(&asset_id, amount),
            })?;

            Self::deposit_event(Event::WithDrawn(who, asset_id, amount));

            Ok(().into())
        }

        /// Deposit assets to the user's account.
        ///
        /// Fails if account is not registered or if the assent is not allowed on the exchange.
        #[pallet::weight(10_000)]
        pub fn deposit(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            ensure!(
                AllowedAssets::<T>::contains_key(&asset_id),
                Error::<T>::AssetNotAllowed
            );

            DepositedAmounts::<T>::try_mutate(&who, |maybe_deposit| match maybe_deposit {
                None => Err(Error::<T>::AccountNotFound),
                Some(deposit) => deposit.add(&asset_id, amount),
            })?;

            Self::deposit_event(Event::Deposited(who, asset_id, amount));

            Ok(().into())
        }

        /// Create a `Basic Pool` with the given supported assets and fee.
        #[pallet::weight(10_000)]
        pub fn add_basic_pool(
            origin: OriginFor<T>,
            assets: Vec<T::AssetId>,
            fee: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // make sure all assets are supporter
            ensure!(
                assets
                    .iter()
                    .all(|asset| AllowedAssets::<T>::contains_key(asset)),
                Error::<T>::AssetNotFound
            );

            // ensure we have still capacity for a new pool
            let mut pool_count = Self::get_number_of_pools();
            ensure!(
                pool_count < T::MaxPoolLimit::get(),
                Error::<T>::PoolLimitReached
            );

            // create a new pool with all the fees and assets
            let pool = BasicPool::new(
                assets.clone(),
                fee + T::ExchangeFee::get() + T::ReferralFee::get(),
                T::ExchangeFee::get(),
                T::ReferralFee::get(),
            )?;

            // increment counter and insert
            pool_count += 1;
            let next_pool_id = T::PoolId::from(pool_count);

            ensure!(
                !Pools::<T>::contains_key(&next_pool_id),
                Error::<T>::InvalidPoolId
            );

            PoolCount::<T>::put(pool_count);
            Pools::<T>::insert(next_pool_id, Pool::BasicPool(pool));

            Self::deposit_event(Event::PoolAdded(who, next_pool_id, assets));

            Ok(().into())
        }

        /// Swap some assets via a series of asset swap actions.
        ///
        /// `referral_account` is an optional account that also should get paid for this swap
        /// according to the pool's referral fee.
        /// For example an intermediary that facilitated this transaction.
        #[pallet::weight(10_000)]
        pub fn swap(
            origin: OriginFor<T>,
            swap_actions: Vec<AssetSwap<T::AssetId, BalanceOf<T>, T::PoolId>>,
            referral_account: Option<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            let mut account_deposit =
                DepositedAmounts::<T>::try_get(&who).map_err(|_| Error::<T>::AccountNotFound)?;

            let mut pools = BTreeMap::new();
            let exchange_account = ExchangeAccount::<T>::get();

            let mut prev_amount = None;
            let mut swapped = Vec::with_capacity(swap_actions.len());

            for swap in swap_actions {
                let AssetSwap {
                    pool_id,
                    asset_in,
                    amount_in,
                    asset_out,
                    min_amount_out,
                } = swap;
                let pool = pools.entry(pool_id).or_insert(
                    Pools::<T>::try_get(&pool_id).map_err(|_| Error::<T>::PoolNotFound)?,
                );

                let amount_in =
                    amount_in.unwrap_or(prev_amount.take().ok_or(Error::<T>::MissingSwapAmountIn)?);

                // withdraw from users account
                account_deposit.sub(&asset_in, amount_in)?;

                let amount_out = pool.swap(
                    &asset_in,
                    amount_in,
                    &asset_out,
                    min_amount_out,
                    exchange_account.as_ref(),
                    referral_account.as_ref(),
                )?;

                account_deposit.add(&asset_out, amount_out)?;

                swapped.push(AssetSwapInfo {
                    pool_id,
                    asset_in,
                    amount_in,
                    asset_out,
                    amount_out,
                });

                prev_amount = Some(amount_out);
            }

            // send back to storage
            DepositedAmounts::<T>::insert(who.clone(), account_deposit);
            for (pool_id, pool) in pools {
                Pools::<T>::insert(pool_id, pool);
            }

            Self::deposit_event(Event::Swapped(who, swapped));

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        // public immutables

        /// Given specific pool, returns amount of `asset_out` received swapping `amount_in` of `asset_in`.
        pub fn get_return(
            pool_id: T::PoolId,
            asset_in: T::AssetId,
            amount_in: BalanceOf<T>,
            asset_out: T::AssetId,
        ) -> Result<BalanceOf<T>, Error<T>> {
            let pool = Pools::<T>::get(&pool_id).ok_or(Error::<T>::PoolNotFound)?;
            pool.get_return(&asset_in, amount_in, &asset_out)
        }

        /// Returns number of pools.
        pub fn get_number_of_pools() -> u64 {
            PoolCount::<T>::get().unwrap_or_default()
        }

        /// Returns information about specified pool if it exists
        pub fn get_pool_info(
            &self,
            pool_id: &T::PoolId,
        ) -> Option<PoolInfo<T::AssetId, BalanceOf<T>>> {
            Pools::<T>::get(pool_id).map(From::from)
        }

        /// Returns the balances of the deposits for given user outside of any pools.
        ///
        /// Returns None if the user is not registered yet.
        pub fn get_deposits(
            &self,
            account_id: &T::AccountId,
        ) -> Option<BTreeMap<T::AssetId, BalanceOf<T>>> {
            DepositedAmounts::<T>::get(account_id).map(|x| x.assets)
        }
    }
}
