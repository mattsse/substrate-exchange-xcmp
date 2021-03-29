use crate::{mock::*, AssetBalance, AssetSwap, Error};
use frame_support::{assert_err, assert_ok};

#[test]
fn change_exchange_beneficiary() {
    new_test_ext().execute_with(|| {
        // set the account for exchange fees
        assert_ok!(AssetExchangeModule::set_exchange_account(Origin::root(), 2));
        assert_eq!(AssetExchangeModule::exchange_account(), Some(2));
    });
}

#[test]
fn allow_assets() {
    new_test_ext().execute_with(|| {
        assert_ok!(AssetExchangeModule::allow_asset(Origin::root(), 5));
        assert_ok!(AssetExchangeModule::allow_asset(Origin::root(), 6));
        assert_ok!(AssetExchangeModule::allow_asset(Origin::root(), 7));
        assert_ok!(AssetExchangeModule::allow_asset(Origin::root(), 8));

        assert!(AssetExchangeModule::allowed_assets(&5).is_some());
        assert!(AssetExchangeModule::allowed_assets(&6).is_some());
        assert!(AssetExchangeModule::allowed_assets(&7).is_some());
        assert!(AssetExchangeModule::allowed_assets(&8).is_some());
    });
}

#[test]
fn register_user_and_deposit_funds() {
    new_test_ext().execute_with(|| {
        // change the owner of the exchange, who receives the exchange fees
        assert_ok!(AssetExchangeModule::set_exchange_account(Origin::root(), 3));

        assert_ok!(AssetExchangeModule::register_user(Origin::signed(1)));
        let deposit = AssetExchangeModule::deposited_amounts(&1).unwrap();
        assert_eq!(deposit.amount, 0);
        assert!(deposit.assets.is_empty());

        // register assets for the user
        assert_ok!(AssetExchangeModule::register_assets(
            Origin::signed(1),
            vec![1, 2]
        ));
        let deposit = AssetExchangeModule::deposited_amounts(&1).unwrap();
        assert_eq!(deposit.assets.get(&1).cloned(), Some(0));
        assert_eq!(deposit.assets.len(), 2);

        // deposit funds in the user's account
        assert_ok!(AssetExchangeModule::deposit(Origin::signed(1), 1, 100_000));
        assert_ok!(AssetExchangeModule::deposit(Origin::signed(1), 2, 100_000));

        let deposit = AssetExchangeModule::deposited_amounts(&1).unwrap();
        assert_eq!(deposit.assets.get(&1).cloned(), Some(100_000));
        assert_eq!(deposit.assets.get(&2).cloned(), Some(100_000));

        // create a new poll
        assert_ok!(AssetExchangeModule::add_basic_pool(
            Origin::signed(1),
            vec![1, 2],
            3
        ));
        assert_eq!(AssetExchangeModule::pool_count(), Some(1));
        assert!(AssetExchangeModule::pools(&1).is_some());

        // add all funds from the user's account into the pool
        assert_ok!(AssetExchangeModule::add_liquidity(
            Origin::signed(1),
            1,
            vec![AssetBalance::new(1, 100_000), AssetBalance::new(2, 100_000)]
        ));

        let deposit = AssetExchangeModule::deposited_amounts(&1).unwrap();
        assert_eq!(deposit.assets.get(&1).cloned(), Some(0));
        assert_eq!(deposit.assets.get(&2).cloned(), Some(0));

        // can't withdraw anymore
        assert_err!(
            AssetExchangeModule::withdraw(Origin::signed(1), 1, 100),
            Error::<Test>::NotEnoughBalance
        );

        assert_ok!(AssetExchangeModule::register_user(Origin::signed(2)));
        // register assets for the user
        assert_ok!(AssetExchangeModule::register_assets(
            Origin::signed(2),
            vec![1, 2]
        ));
        assert_ok!(AssetExchangeModule::deposit(Origin::signed(2), 1, 100_000));
        assert_ok!(AssetExchangeModule::deposit(Origin::signed(2), 2, 100_000));

        // swap assets from user 2's account
        assert_ok!(AssetExchangeModule::swap(
            Origin::signed(2),
            vec![AssetSwap {
                pool_id: 1,
                asset_in: 1,
                amount_in: Some(10_000),
                asset_out: 2,
                min_amount_out: 9000
            }],
            None
        ));

        // ensure swapped assets are back in users account
        let deposit = AssetExchangeModule::deposited_amounts(&2).unwrap();
        assert_eq!(deposit.assets.get(&1).cloned(), Some(90_000));

        // remove all liquidity from the pool
        let pool = AssetExchangeModule::pools(&1).unwrap();
        let share_balance = pool.share_balances(&1).cloned().unwrap();
        assert_ok!(AssetExchangeModule::remove_liquidity(
            Origin::signed(1),
            1,
            share_balance,
            Vec::new()
        ));

        let deposit_liquidity_provider = AssetExchangeModule::deposited_amounts(&1).unwrap();

        // remove all liquidity from the exchange beneficiary's account
        let share_balance = pool.share_balances(&3).cloned().unwrap();
        assert_ok!(AssetExchangeModule::remove_liquidity(
            Origin::signed(3),
            1,
            share_balance,
            Vec::new()
        ));

        let exchange_deposit = AssetExchangeModule::deposited_amounts(&3).unwrap();
        // balance of exchange and provider liquidity provider should match swapped volume
        assert_eq!(
            exchange_deposit.assets.get(&1).cloned().unwrap()
                + deposit_liquidity_provider.assets.get(&1).cloned().unwrap(),
            110_000
        );

        let swapper_deposit = AssetExchangeModule::deposited_amounts(&2).unwrap();

        // asset balances should match total deposited volume
        assert_eq!(
            exchange_deposit.assets.get(&1).cloned().unwrap()
                + deposit_liquidity_provider.assets.get(&1).cloned().unwrap()
                + swapper_deposit.assets.get(&1).cloned().unwrap(),
            200_000
        );
        assert_eq!(
            exchange_deposit.assets.get(&2).cloned().unwrap()
                + deposit_liquidity_provider.assets.get(&2).cloned().unwrap()
                + swapper_deposit.assets.get(&2).cloned().unwrap(),
            200_000
        );
    });
}
