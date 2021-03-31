//! # Asset Exchange xcmp support
//!
//! Provides support for sending and handling cross-chain asset transfer of assets
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::sp_runtime::traits::{CheckedConversion, Convert};
use frame_support::traits::Get;
use xcm::v0::{Error as XcmError, Junction, MultiAsset, MultiLocation, Result as XcmResult};
use xcm_executor::traits::{LocationConversion, MatchesFungible, TransactAsset};

pub use pallet::*;
use pallet_asset_exchange::BalanceOf;
use sp_std::convert::TryFrom;
use sp_std::marker::PhantomData;
use sp_std::vec::Vec;

/// Convert `MultiAsset` to `AssetId`.
pub trait AssetIdConversion<AssetId> {
    /// Get `AssetId` from `MultiAsset`. Returns `None` if conversion failed.
    fn from_asset(asset: &MultiAsset) -> Option<AssetId>;
}

/// `AssetIdConversion` implementation. Converts relay chain tokens, or
/// parachain assets that could be decoded from a general key.
pub struct AssetIdConverter<AssetId, RelayChainAssetId>(
    PhantomData<AssetId>,
    PhantomData<RelayChainAssetId>,
);
impl<AssetId, RelayChainAssetId> AssetIdConversion<AssetId>
    for AssetIdConverter<AssetId, RelayChainAssetId>
where
    AssetId: TryFrom<Vec<u8>>,
    RelayChainAssetId: Get<AssetId>,
{
    fn from_asset(asset: &MultiAsset) -> Option<AssetId> {
        if let MultiAsset::ConcreteFungible { id: location, .. } = asset {
            if location == &MultiLocation::X1(Junction::Parent) {
                return Some(RelayChainAssetId::get());
            }
            if let Some(Junction::GeneralKey(key)) = location.last() {
                return AssetId::try_from(key.clone()).ok();
            }
        }
        None
    }
}

/// Helper Converter
pub struct U128BalanceConverter;

impl Convert<u128, u128> for U128BalanceConverter {
    fn convert(a: u128) -> u128 {
        a
    }
}

#[frame_support::pallet]
pub mod pallet {
    use cumulus_primitives_core::ParaId;
    use frame_support::sp_runtime::traits::Convert;
    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    use sp_std::prelude::*;
    use xcm::v0::{ExecuteXcm, Junction, MultiAsset, MultiLocation, NetworkId, Order, Xcm};
    use xcm_executor::traits::LocationConversion;

    use pallet_asset_exchange::BalanceOf;

    use crate::AssetIdConversion;

    /// Type, used for representation of assets, located on other parachains (both internal and remote).
    pub type AssetIdOf<T> = <T as pallet_asset_exchange::Config>::AssetId;

    #[derive(Encode, Decode, Eq, PartialEq, Clone, Copy, RuntimeDebug)]
    /// Identity of chain.
    pub enum ChainId {
        /// The relay chain.
        RelayChain,
        /// A parachain.
        ParaChain(ParaId),
    }

    #[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug)]
    /// Identity of cross chain currency.
    pub struct XAssetId {
        /// The reserve chain of the currency. For instance, the reserve chain
        /// of DOT is Polkadot.
        pub chain_id: ChainId,
        /// The identity of the currency.
        pub currency_id: Vec<u8>,
    }

    impl Into<MultiLocation> for XAssetId {
        fn into(self) -> MultiLocation {
            MultiLocation::X1(Junction::GeneralKey(self.currency_id))
        }
    }

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_asset_exchange::Config {
        type Event: From<Event<Self>>
            + IsType<<Self as frame_system::Config>::Event>
            + IsType<<Self as pallet_asset_exchange::Config>::Event>;

        /// Something to execute an XCM message.
        type XcmExecutor: ExecuteXcm;

        /// Used to convert relay chain decimals to local asset pallet balance type
        type FromRelayChainBalance: Convert<u128, BalanceOf<Self>>;

        type ToRelayChainBalance: Convert<BalanceOf<Self>, u128>;

        /// How to convert locations
        type AccountIdConverter: LocationConversion<Self::AccountId>;

        /// How to convert Multiassets to an AssetId
        type AssetIdConverter: AssetIdConversion<AssetIdOf<Self>>;

        type AssetIdLocationConverter: Convert<AssetIdOf<Self>, MultiLocation>;

        /// Convert `Self::Account` to `AccountId32`
        type AccountId32Convert: Convert<Self::AccountId, [u8; 32]>;

        /// The network id of relay chain. Typically `NetworkId::Polkadot` or
        /// `NetworkId::Kusama`.
        type RelayChainNetworkId: Get<NetworkId>;

        /// The relaychain's asset ID.
        type RelayChainAssetId: Get<AssetIdOf<Self>>;

        /// Own parachain ID.
        type ParaId: Get<ParaId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn something)]
    pub type Something<T> = StorageValue<_, u32>;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Transferred relay chain asset to relay chain. [src, dest, amount]
        TransferredToRelayChain(T::AccountId, T::AccountId, BalanceOf<T>),

        /// Transferred to parachain. [asset_id, src, para_id, dest, dest_network, amount]
        TransferredToParachain(
            T::AssetId,
            T::AccountId,
            ParaId,
            MultiLocation,
            BalanceOf<T>,
        ),

        /// Transferred custom asset to the account from the given para chain account.
        DepositAssetViaXCMP(
            ParaId,
            AssetIdOf<T>,
            T::AccountId,
            AssetIdOf<T>,
            BalanceOf<T>,
        ),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Thrown when no `MultiLocation` could be derived from the sender
        BadXcmOrigin,
        /// Failed to send XCM message.
        FailedToSend,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transfer `amount` of main currency on the relay chain to
        /// the given `dest` account.
        #[pallet::weight(10_000)]
        fn transfer_balance_to_relay_chain(
            origin: OriginFor<T>,
            dest: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            let origin = T::AccountIdConverter::try_into_location(who.clone())
                .map_err(|_| Error::<T>::BadXcmOrigin)?;

            // construct the xcm message
            let xcm = Xcm::WithdrawAsset {
                assets: vec![MultiAsset::ConcreteFungible {
                    id: MultiLocation::X1(Junction::Parent),
                    amount: T::ToRelayChainBalance::convert(amount),
                }],
                effects: vec![Order::InitiateReserveWithdraw {
                    assets: vec![MultiAsset::All],
                    reserve: MultiLocation::X1(Junction::Parent),
                    effects: vec![Order::DepositAsset {
                        assets: vec![MultiAsset::All],
                        dest: MultiLocation::X1(Junction::AccountId32 {
                            network: T::RelayChainNetworkId::get(),
                            id: T::AccountId32Convert::convert(dest.clone()),
                        }),
                    }],
                }],
            };

            // execute the message
            T::XcmExecutor::execute_xcm(origin, xcm).map_err(|_| Error::<T>::FailedToSend)?;

            Self::deposit_event(Event::<T>::TransferredToRelayChain(who, dest, amount));

            Ok(().into())
        }

        /// Transfer a given `amount` of another parachain asset to another parachain.
        #[pallet::weight(10_000)]
        pub fn transfer_balance_to_parachain(
            origin: OriginFor<T>,
            asset_id: AssetIdOf<T>,
            para_id: ParaId,
            dest: MultiLocation,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            let origin = T::AccountIdConverter::try_into_location(who.clone())
                .map_err(|_| Error::<T>::BadXcmOrigin)?;

            if para_id == T::ParaId::get() {
                // nothing to do when target is own chain
                return Ok(().into());
            }

            let xcm = if asset_id == T::RelayChainAssetId::get() {
                // relay chain
                Self::transfer_relay_chain_assets_to_parachain(para_id, dest.clone(), amount)
            } else {
                Self::transfer_asset_to_parachain(asset_id.clone(), para_id, dest.clone(), amount)
            };

            // send xcm
            T::XcmExecutor::execute_xcm(origin, xcm).map_err(|_| Error::<T>::FailedToSend)?;

            Self::deposit_event(Event::<T>::TransferredToParachain(
                asset_id, who, para_id, dest, amount,
            ));

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        fn transfer_relay_chain_assets_to_parachain(
            para_id: ParaId,
            dest: MultiLocation,
            amount: BalanceOf<T>,
        ) -> Xcm {
            Xcm::WithdrawAsset {
                assets: vec![MultiAsset::ConcreteFungible {
                    id: MultiLocation::X1(Junction::Parent),
                    amount: T::ToRelayChainBalance::convert(amount),
                }],
                effects: vec![Order::InitiateReserveWithdraw {
                    assets: vec![MultiAsset::All],
                    reserve: MultiLocation::X1(Junction::Parent),
                    effects: vec![Order::DepositReserveAsset {
                        assets: vec![MultiAsset::All],
                        // Reserve asset deposit dest is children parachain(of parent).
                        dest: MultiLocation::X1(Junction::Parachain { id: para_id.into() }),
                        effects: vec![Order::DepositAsset {
                            assets: vec![MultiAsset::All],
                            dest,
                        }],
                    }],
                }],
            }
        }

        /// Transfer parachain assets "owned" by self parachain to another
        /// parachain.
        ///
        /// NOTE - `para_id` must not be self parachain.
        fn transfer_asset_to_parachain(
            asset_id: AssetIdOf<T>,
            para_id: ParaId,
            dest: MultiLocation,
            amount: BalanceOf<T>,
        ) -> Xcm {
            Xcm::WithdrawAsset {
                assets: vec![MultiAsset::ConcreteFungible {
                    id: T::AssetIdLocationConverter::convert(asset_id),
                    amount: T::ToRelayChainBalance::convert(amount),
                }],
                effects: vec![Order::DepositReserveAsset {
                    assets: vec![MultiAsset::All],
                    dest: MultiLocation::X2(
                        Junction::Parent,
                        Junction::Parachain { id: para_id.into() },
                    ),
                    effects: vec![Order::DepositAsset {
                        assets: vec![MultiAsset::All],
                        dest,
                    }],
                }],
            }
        }
    }
}
/// Asset transaction errors.
enum Error {
    /// Failed to match asset.
    FailedToMatchAsset,
    /// `MultiLocation` to `AccountId` Conversion failed.
    AccountIdConversionFailed,
    /// `AssetId` conversion failed.
    AssetIdConversionFailed,
    /// Failed to withdraw due lack of funds
    NotEnoughBalance,
    /// Failed to add to balance
    BalanceOverflow,
}

impl From<Error> for XcmError {
    fn from(err: Error) -> Self {
        match err {
            Error::FailedToMatchAsset => XcmError::FailedToTransactAsset("FailedToMatchAsset"),
            Error::AccountIdConversionFailed => {
                XcmError::FailedToTransactAsset("AccountIdConversionFailed")
            }
            Error::AssetIdConversionFailed => {
                XcmError::FailedToTransactAsset("AssetIdConversionFailed")
            }
            Error::NotEnoughBalance => XcmError::FailedToTransactAsset("NotEnoughBalance"),
            Error::BalanceOverflow => XcmError::FailedToTransactAsset("BalanceOverflow"),
        }
    }
}

impl<T: Config> MatchesFungible<BalanceOf<T>> for Pallet<T> {
    fn matches_fungible(a: &MultiAsset) -> Option<BalanceOf<T>> {
        if let MultiAsset::ConcreteFungible { id, amount } = a {
            if id == &MultiLocation::X1(Junction::Parent) {
                // Convert relay chain decimals to local chain
                let local_amount = T::FromRelayChainBalance::convert(*amount);
                return CheckedConversion::checked_from(local_amount);
            }
            if let Some(Junction::GeneralKey(_)) = id.last() {
                // TODO should validate that key is valid asset id in the exchange store?
                return CheckedConversion::checked_from(*amount);
            }
        }
        None
    }
}

impl<T: Config> TransactAsset for Pallet<T> {
    fn deposit_asset(what: &MultiAsset, location: &MultiLocation) -> XcmResult {
        let who = T::AccountIdConverter::from_location(location)
            .ok_or_else(|| XcmError::from(Error::AccountIdConversionFailed))?;
        let asset_id = T::AssetIdConverter::from_asset(what)
            .ok_or_else(|| XcmError::from(Error::AssetIdConversionFailed))?;
        let amount = Self::matches_fungible(what)
            .ok_or_else(|| XcmError::from(Error::FailedToMatchAsset))?;

        // deposit into user's account
        <pallet_asset_exchange::Pallet<T>>::deposit_asset_balance(&who, &asset_id, amount)
            .map_err(|_| XcmError::from(Error::BalanceOverflow))?;

        Ok(())
    }

    fn withdraw_asset(what: &MultiAsset, location: &MultiLocation) -> Result<MultiAsset, XcmError> {
        let who = T::AccountIdConverter::from_location(location)
            .ok_or_else(|| XcmError::from(Error::AccountIdConversionFailed))?;
        let asset_id = T::AssetIdConverter::from_asset(what)
            .ok_or_else(|| XcmError::from(Error::AssetIdConversionFailed))?;
        let amount = Self::matches_fungible(what)
            .ok_or_else(|| XcmError::from(Error::FailedToMatchAsset))?;

        // withdraw from the user's account
        <pallet_asset_exchange::Pallet<T>>::withdraw_asset_balance(&who, &asset_id, amount)
            .map_err(|_| XcmError::from(Error::NotEnoughBalance))?;

        Ok(what.clone())
    }
}
