use common_structs::TokenPair;
use fastloan_setup::{setup, create_pool, FASTLOAN_ADDRESS_EXPR, POOL_ADDRESS_EXPR, OWNER_ADDRESS_EXPR};
use fees_collector::fees_accumulation::FeesAccumulationModule;
use multiversx_sc::types::{MultiValueEncoded, EsdtTokenPayment};
use multiversx_sc_scenario::{*, scenario_model::{ScCallStep, CheckStateStep, CheckAccount, AddressValue, TxExpect}};
use fastloan::{self, FastLoanContract, types::{Borrow, Exchange, UserAction}, setup::SetupModule};
use pair::{config::ConfigModule, Pair, flashloan::FlashLoanModule};

use crate::fastloan_setup::{MEX_TOKEN_ID, WEGLD_TOKEN_ID};
use crate::fastloan::types::SwapContractAddress;


pub mod fastloan_setup;

#[test]
fn try_setup() {
    setup(
        100u64
    );
}

// Borrow 10 WEGLD
// Give back 10 WEGLD
// No action
// No fees
#[test]
fn borrow_give_back_no_fee() {
    let (
        mut world,
        pool_whitebox,
        fastloan_whitebox,
        owner_address,
        _fees_collector_whitebox
    ) = setup(
        0u64
    );

    world.whitebox_call( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address), 
        |sc| {
            let borrow = Borrow { 
                token : managed_token_id!(WEGLD_TOKEN_ID),
                amount: managed_biguint!(100)
            };
            let mut borrows = MultiValueEncoded::new();
            borrows.push(borrow);
            sc.borrow(borrows, MultiValueEncoded::new());
        }
    );

    world.check_state_step(
        CheckStateStep::new()
            .put_account("address:owner", 
            CheckAccount::new()
            .esdt_balance(
                WEGLD_TOKEN_ID.to_vec(),
                "9999990000")
        )
    );

    world.whitebox_query(
        &pool_whitebox, 
        |sc| {
            let wegld_reserve = sc.pair_reserve(&managed_token_id!(WEGLD_TOKEN_ID)).get();
            let mex_reserve = sc.pair_reserve(&managed_token_id!(MEX_TOKEN_ID)).get();
            assert_eq!(wegld_reserve, managed_biguint!(10000));
            assert_eq!(mex_reserve, managed_biguint!(10000))
    });

}


// Borrow 10 WEGLD and 10 MEX
// Give back 10 WEGLD and 10 MEX
// No action
// No fees
#[test]
fn borrow_two_tokens_give_back_no_fee() {
    let (
        mut world,
        pool_whitebox,
        fastloan_whitebox,
        owner_address,
        _fees_collector_whitebox
    ) = setup(
        0u64
    );

    world.whitebox_call( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address), 
        |sc| {
            let borrow_wegld = Borrow { 
                token : managed_token_id!(WEGLD_TOKEN_ID),
                amount: managed_biguint!(100)
            };
            let borrow_mex = Borrow { 
                token : managed_token_id!(MEX_TOKEN_ID),
                amount: managed_biguint!(100)
            };
            let mut borrows = MultiValueEncoded::new();
            borrows.push(borrow_mex);
            borrows.push(borrow_wegld);
            sc.borrow(borrows, MultiValueEncoded::new());
        }
    );

    world.check_state_step(
        CheckStateStep::new()
            .put_account("address:owner", 
            CheckAccount::new()
            .esdt_balance(
                WEGLD_TOKEN_ID.to_vec(),
                "9999990000")
        )
    );

    world.whitebox_query(
        &pool_whitebox, 
        |sc| {
            let wegld_reserve = sc.pair_reserve(&managed_token_id!(WEGLD_TOKEN_ID)).get();
            let mex_reserve = sc.pair_reserve(&managed_token_id!(MEX_TOKEN_ID)).get();
            assert_eq!(wegld_reserve, managed_biguint!(10000));
            assert_eq!(mex_reserve, managed_biguint!(10000))
    });

}


#[test]
fn borrow_and_swap_other_pool_swap_back() {
    let (
        mut world,
        _pool_whitebox,
        fastloan_whitebox,
        owner_address,
        _fees_collector_whitebox
    ) = setup(
        100u64
    );

    let pool_expr = "sc:pool-ash";
    let owner_address_expr = "address:owner-ash";
    let first_token  : &[u8] = b"ASH-123456";
    let second_token : &[u8] = WEGLD_TOKEN_ID;
    let lp_token = b"LPTOKASH-abcdef";


    let pool_ashswap = create_pool(
        &mut world,
        pool_expr,
        owner_address_expr,
        first_token,
        second_token,
        lp_token,
        0u64
    );

     world.whitebox_call(
        &pool_ashswap,
        ScCallStep::new()
        .from(owner_address_expr)
        .to(pool_expr)
        .esdt_transfer(first_token, 0, rust_biguint!(10000))
        .esdt_transfer(second_token, 0, rust_biguint!(10000))
        .no_expect(), 
        |sc| {
            sc.add_liquidity(rust_biguint!(10000).into(), rust_biguint!(10000).into());
            sc.set_flash_loan_sc(managed_address!(&AddressValue::from(FASTLOAN_ADDRESS_EXPR).to_address()));
        }
    );

    // Setup 
    world.whitebox_call( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address), 
        |sc| {
            let mut multi_value = MultiValueEncoded::new();
            let pair = TokenPair {
                first_token : managed_token_id!(WEGLD_TOKEN_ID),
                second_token : managed_token_id!(MEX_TOKEN_ID)
            };
            let second_pair : TokenPair<DebugApi> = TokenPair {
                first_token : managed_token_id!(first_token),
                second_token : managed_token_id!(second_token)
            };
            let pool = (Exchange::Xexchange, SwapContractAddress {
                address : managed_address!(&AddressValue::from(POOL_ADDRESS_EXPR).to_address()),
                tokens: pair
            });
            let second_pool = (Exchange::Ashswap, SwapContractAddress {
                address : managed_address!(&AddressValue::from(pool_expr).to_address()),
                tokens: second_pair
            });
            multi_value.push(pool);
            multi_value.push(second_pool);
            sc.init(
                100u64
            );
            sc.owner_setup_pool_addresses_pairs(multi_value);
        }
    );

    // Swap WEGLD to ASH and not enough WEGLD to give back
    world.whitebox_call_check( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address)
        .expect(TxExpect::user_error("str:One of the borrowed token amount was not entirely given back")), 
        |sc| {
            let borrow_wegld = Borrow { 
                token : managed_token_id!(WEGLD_TOKEN_ID),
                amount: managed_biguint!(10)
            };
            let borrow_mex = Borrow { 
                token : managed_token_id!(MEX_TOKEN_ID),
                amount: managed_biguint!(10)
            };
            let mut borrows = MultiValueEncoded::new();
            borrows.push(borrow_mex);
            borrows.push(borrow_wegld);

            let mut actions = MultiValueEncoded::new();
            let action1 = UserAction {
                exchange : Exchange::Ashswap,
                first_token : managed_token_id!(second_token),
                second_token : managed_token_id!(first_token),
                amount_swap : managed_biguint!(10)
            };
            actions.push(action1);
            sc.borrow(borrows, actions);
        },
        |r| {
            r.assert_error(4, "One of the borrowed token amount was not entirely given back")
        }
    );

    // Swap WEGLD to ASH and swap back again on the same pool
    // But cannot have the starting amount (swap fees)
    world.whitebox_call_check( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address)
        .expect(TxExpect::user_error("str:One of the borrowed token amount was not entirely given back")), 
        |sc| {
            let borrow_wegld = Borrow { 
                token : managed_token_id!(WEGLD_TOKEN_ID),
                amount: managed_biguint!(1000)
            };
            let borrow_mex = Borrow { 
                token : managed_token_id!(MEX_TOKEN_ID),
                amount: managed_biguint!(1000)
            };
            let mut borrows = MultiValueEncoded::new();
            borrows.push(borrow_mex);
            borrows.push(borrow_wegld);

            let mut actions = MultiValueEncoded::new();
            let action1 = UserAction {
                exchange : Exchange::Ashswap,
                first_token : managed_token_id!(second_token),
                second_token : managed_token_id!(first_token),
                amount_swap : managed_biguint!(1000)
            };
            let action2 = UserAction {
                exchange : Exchange::Ashswap,
                first_token : managed_token_id!(first_token),
                second_token : managed_token_id!(second_token),
                amount_swap : managed_biguint!(906)
            };
            actions.push(action1);
            actions.push(action2);
            sc.borrow(borrows, actions);
        },
        |r| {
            r.assert_error(4, "One of the borrowed token amount was not entirely given back")
        }
    );


}

#[test]
fn arbitrage_between_two_differents_exchanges_pools_fees() {
    let (
        mut world,
        _pool_whitebox,
        fastloan_whitebox,
        owner_address,
        fees_collector_whitebox
    ) = setup(
        100u64
    );


    // Reverse pool on Ashswap with difference
    let pool_expr = "sc:pool-reverse";
    let owner_address_reverse_expr = "address:owner-reverse";
    let first_token  : &[u8] = MEX_TOKEN_ID;
    let second_token : &[u8] = WEGLD_TOKEN_ID;
    let lp_token = b"LPREV-abcdef";

    let pool_reverse = create_pool(
        &mut world,
        pool_expr,
        owner_address_reverse_expr,
        first_token,
        second_token,
        lp_token,
        0u64
    );

     // Normal pool on Jexchange
     let pool_jex_expr = "sc:pool-jex";
     let owner_address_jex_expr = "address:owner-jex";
     let lp_token_jex = b"LPMEG-123456";
 
     let pool_jex = create_pool(
         &mut world,
         pool_jex_expr,
         owner_address_jex_expr,
         second_token,
         first_token,
         lp_token_jex,
         0u64
     );

     // Setup Ashswap reverse pool
     world.whitebox_call(
        &pool_reverse,
        ScCallStep::new()
        .from(owner_address_reverse_expr)
        .to(pool_expr)
        .esdt_transfer(first_token, 0, rust_biguint!(1500))
        .esdt_transfer(second_token, 0, rust_biguint!(10000))
        .no_expect(), 
        |sc| {
            sc.add_liquidity(rust_biguint!(1500).into(), rust_biguint!(10000).into());
            sc.set_flash_loan_sc(managed_address!(&AddressValue::from(FASTLOAN_ADDRESS_EXPR).to_address()));
        }
    );

    // Setup jexchange normal pool
    world.whitebox_call(
        &pool_jex,
        ScCallStep::new()
        .from(owner_address_jex_expr)
        .to(pool_jex_expr)
        .esdt_transfer(second_token, 0, rust_biguint!(10000))
        .esdt_transfer(first_token, 0, rust_biguint!(10000))
        .no_expect(), 
        |sc| {
            sc.add_liquidity(rust_biguint!(10000).into(), rust_biguint!(10000).into());
            sc.set_flash_loan_sc(managed_address!(&AddressValue::from(FASTLOAN_ADDRESS_EXPR).to_address()));
        }
    );

    // Setup Fastloan SC
    world.whitebox_call( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address), 
        |sc| {
            let mut multi_value = MultiValueEncoded::new();
            let pair = TokenPair {
                first_token : managed_token_id!(second_token),
                second_token : managed_token_id!(first_token)
            };
            let second_pair : TokenPair<DebugApi> = TokenPair {
                first_token : managed_token_id!(first_token),
                second_token : managed_token_id!(second_token)
            };
            let third_pair = TokenPair {
                first_token : managed_token_id!(second_token),
                second_token : managed_token_id!(first_token)
            };
            let pool = (Exchange::Xexchange, SwapContractAddress {
                address : managed_address!(&AddressValue::from(POOL_ADDRESS_EXPR).to_address()),
                tokens: pair
            });
            let second_pool = (Exchange::Ashswap, SwapContractAddress {
                address : managed_address!(&AddressValue::from(pool_expr).to_address()),
                tokens: second_pair
            });
            let third_pool = (Exchange::Jexchange, SwapContractAddress {
                address : managed_address!(&AddressValue::from(pool_jex_expr).to_address()),
                tokens: third_pair
            });
            multi_value.push(pool);
            multi_value.push(second_pool);
            multi_value.push(third_pool);
            sc.init(
                100u64
            );
            sc.owner_setup_pool_addresses_pairs(multi_value);
        }
    );

    world.check_state_step(
        CheckStateStep::new()
        .put_account(OWNER_ADDRESS_EXPR, CheckAccount::new()
        .esdt_balance(second_token.to_vec(), "9999990000")
    ));

    // Try arbitrage but borrow and swap same xExchange pool error
    world.whitebox_call_check( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address)
        .expect(TxExpect::user_error("str:Cannot call a pool we borrowed from to swap")), 
        |sc| {
            let borrow_wegld = Borrow { 
                token : managed_token_id!(second_token),
                amount: managed_biguint!(1000)
            };
            let mut borrows = MultiValueEncoded::new();
            borrows.push(borrow_wegld);

            let mut actions = MultiValueEncoded::new();
            let action1 = UserAction {
                exchange : Exchange::Xexchange,
                first_token : managed_token_id!(second_token),
                second_token : managed_token_id!(first_token),
                amount_swap : managed_biguint!(1000)
            };
            let action2 = UserAction {
                exchange : Exchange::Ashswap,
                first_token : managed_token_id!(first_token),
                second_token : managed_token_id!(second_token),
                amount_swap : managed_biguint!(997)
            };
            actions.push(action1);
            actions.push(action2);
            sc.borrow(borrows, actions);
        },
        |r| {
            r.assert_user_error("Cannot call a pool we borrowed from to swap");
        }
    );

    // Borrow 1000 WEGLD
    // Swap on Jexchange for 906 MEX
    // Swap 906 MEX for 3758 WEGLD
    // Give 1010 to Pool
    // Send 10 to fees from pool
    // Give 2748 to User 
    world.whitebox_call_check( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address)
        .expect(TxExpect::ok()), 
        |sc| {
            let borrow_wegld = Borrow { 
                token : managed_token_id!(second_token),
                amount: managed_biguint!(1000)
            };
            let mut borrows = MultiValueEncoded::new();
            borrows.push(borrow_wegld);

            let mut actions = MultiValueEncoded::new();
            let action1 = UserAction {
                exchange : Exchange::Jexchange,
                first_token : managed_token_id!(second_token),
                second_token : managed_token_id!(first_token),
                amount_swap : managed_biguint!(1000)
            };
            let action2 = UserAction {
                exchange : Exchange::Ashswap,
                first_token : managed_token_id!(first_token),
                second_token : managed_token_id!(second_token),
                amount_swap : managed_biguint!(906)
            };
            actions.push(action1);
            actions.push(action2);
            sc.borrow(borrows, actions);
        },
        |r| {
            r.assert_ok()
        }
    );

    // Check correct user balance
    world.check_state_step(
        CheckStateStep::new()
        .put_account(OWNER_ADDRESS_EXPR, CheckAccount::new()
        .esdt_balance(second_token.to_vec(), "9999992748")
    ));

    // Check 10 WEGLD fees in fee-collector
    world.whitebox_query(&fees_collector_whitebox, |sc| {
        let accumulated = sc.accumulated_fees(1, &managed_token_id!(second_token)).get();
        assert_eq!(accumulated, managed_biguint!(10));
    });
}

#[test]
fn borrow_to_not_set_exchange_pool() {
    let (
        mut world,
        _pool_whitebox,
        fastloan_whitebox,
        owner_address,
        _fees_collector_whitebox
    ) = setup(
        100u64
    );

    world.whitebox_call_check( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address)
        .expect(TxExpect::user_error("str:Could not execute all borrows")), 
        |sc| {
            let borrow_imaginary_token = Borrow { 
                token : managed_token_id!("IMG-123456"),
                amount: managed_biguint!(100)
            };
            let mut borrows = MultiValueEncoded::new();
            borrows.push(borrow_imaginary_token);
            sc.borrow(borrows, MultiValueEncoded::new());
        },
        |r| r.assert_error(4, "Could not execute all borrows")
    );
}

#[test]
fn borrow_more_than_pool_reserve() {
    let (
        mut world,
        _pool_whitebox,
        fastloan_whitebox,
        owner_address,
        _fees_collector_whitebox
    ) = setup(
        100u64
    );

    world.whitebox_call_check( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address)
        .expect(TxExpect::user_error("str:Not enough reserve")), 
        |sc| {
            let borrow = Borrow { 
                token : managed_token_id!(WEGLD_TOKEN_ID),
                amount: managed_biguint!(1000000000000)
            };
            let mut borrows = MultiValueEncoded::new();
            borrows.push(borrow);
            sc.borrow(borrows, MultiValueEncoded::new());
        },
        |r| r.assert_error(4, "Not enough reserve")
    );
}

#[test]
fn pool_endpoints_unauthorized() {
    let (
        mut world,
        pool_whitebox,
        _fastloan_whitebox,
        owner_address,
        _fees_collector_whitebox
    ) = setup(
        100u64
    );

    world.whitebox_call_check( 
        &pool_whitebox, 
        ScCallStep::new() 
        .from(&owner_address)
        .expect(TxExpect::user_error("str:Caller is not flash loan SC")), 
        |sc| {
            sc.borrow_liquidity(
                EsdtTokenPayment {
                    token_identifier : managed_token_id!(WEGLD_TOKEN_ID),
                    token_nonce : 0,
                    amount : managed_biguint!(100)
                }, 1u64
            );
        },
        |r| r.assert_error(4, "Caller is not flash loan SC")
    );

    world.whitebox_call_check( 
        &pool_whitebox, 
        ScCallStep::new() 
        .from(&owner_address)
        .expect(TxExpect::user_error("str:Caller is not flash loan SC")), 
        |sc| {
            sc.repay(1u64);
        },
        |r| r.assert_error(4, "Caller is not flash loan SC")
    );
}

#[test]
fn pool_borrow_incorrect_token() {
    let (
        mut world,
        pool_whitebox,
        _fastloan_whitebox,
        _owner_address,
        _fees_collector_whitebox
    ) = setup(
        100u64
    );

    world.whitebox_call_check( 
        &pool_whitebox, 
        ScCallStep::new() 
        .from(FASTLOAN_ADDRESS_EXPR)
        .expect(TxExpect::user_error("str:Bad payment tokens")), 
        |sc| {
            sc.borrow_liquidity(
                EsdtTokenPayment {
                    token_identifier : managed_token_id!("ASH-123456"),
                    token_nonce : 0,
                    amount : managed_biguint!(100)
                }, 1u64
            );
        },
        |r| r.assert_error(4, "Bad payment tokens")
    );

}

#[test]
fn fastloan_deposit_from_unauthorized() {
    let (
        mut world,
        _pool_whitebox,
        fastloan_whitebox,
        owner_address,
        _fees_collector_whitebox
    ) = setup(
        100u64
    );

    world.whitebox_call_check( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address)
        .esdt_transfer(WEGLD_TOKEN_ID, 0, rust_biguint!(100))
        .expect(TxExpect::user_error("str:Caller address is not defined")), 
        |sc| {
            sc.deposit_loan(1u64)
        },
        |r| r.assert_error(4, "Caller address is not defined")
    );

}

#[test]
fn user_action_to_unknown_contract() {
    let (
        mut world,
        _pool_whitebox,
        fastloan_whitebox,
        owner_address,
        _fees_collector_whitebox
    ) = setup(
        100u64
    );

    world.whitebox_call_check( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address)
        .expect(TxExpect::user_error("str:A swap pair is unavailable for the action selected")), 
        |sc| {
            let borrow_wegld = Borrow { 
                token : managed_token_id!(WEGLD_TOKEN_ID),
                amount: managed_biguint!(1000)
            };
            let mut borrows = MultiValueEncoded::new();
            borrows.push(borrow_wegld);

            let mut actions = MultiValueEncoded::new();
            let action1 = UserAction {
                exchange : Exchange::JewelSwap,
                first_token : managed_token_id!(MEX_TOKEN_ID),
                second_token : managed_token_id!(WEGLD_TOKEN_ID),
                amount_swap : managed_biguint!(1000)
            };
            let action2 = UserAction {
                exchange : Exchange::Jexchange,
                first_token : managed_token_id!(WEGLD_TOKEN_ID),
                second_token : managed_token_id!(MEX_TOKEN_ID),
                amount_swap : managed_biguint!(997)
            };
            actions.push(action1);
            actions.push(action2);
            sc.borrow(borrows, actions);
        },
        |r| {
            r.assert_error(4, "A swap pair is unavailable for the action selected");
        }
    );
}


#[test]
fn pool_borrow_and_swap_same_pool() {
    let (
        mut world,
        _pool_whitebox,
        fastloan_whitebox,
        owner_address,
        _fees_collector_whitebox
    ) = setup(
        100u64
    );

    let pool_expr = "sc:pool-ash";
    let owner_address_expr = "address:owner-ash";
    let first_token  : &[u8] = MEX_TOKEN_ID;
    let second_token : &[u8] = WEGLD_TOKEN_ID;
    let lp_token = b"LPTOKASH-abcdef";


    let pool_ashswap = create_pool(
        &mut world,
        pool_expr,
        owner_address_expr,
        first_token,
        second_token,
        lp_token,
        0u64
    );

     world.whitebox_call(
        &pool_ashswap,
        ScCallStep::new()
        .from(owner_address_expr)
        .to(pool_expr)
        .esdt_transfer(first_token, 0, rust_biguint!(10000))
        .esdt_transfer(second_token, 0, rust_biguint!(10000))
        .no_expect(), 
        |sc| {
            sc.add_liquidity(rust_biguint!(10000).into(), rust_biguint!(10000).into());
            sc.set_flash_loan_sc(managed_address!(&AddressValue::from(FASTLOAN_ADDRESS_EXPR).to_address()));
        }
    );

    // Setup 
    world.whitebox_call( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address), 
        |sc| {
            let mut multi_value = MultiValueEncoded::new();
            let pair = TokenPair {
                first_token : managed_token_id!(WEGLD_TOKEN_ID),
                second_token : managed_token_id!(MEX_TOKEN_ID)
            };
            let second_pair : TokenPair<DebugApi> = TokenPair {
                first_token : managed_token_id!(first_token),
                second_token : managed_token_id!(second_token)
            };
            let pool = (Exchange::Xexchange, SwapContractAddress {
                address : managed_address!(&AddressValue::from(POOL_ADDRESS_EXPR).to_address()),
                tokens: pair
            });
            let second_pool = (Exchange::Ashswap, SwapContractAddress {
                address : managed_address!(&AddressValue::from(pool_expr).to_address()),
                tokens: second_pair
            });
            multi_value.push(pool);
            multi_value.push(second_pool);
            sc.init(
                100u64
            );
            sc.owner_setup_pool_addresses_pairs(multi_value);
        }
    );

    

    world.whitebox_call_check( 
        &fastloan_whitebox, 
        ScCallStep::new() 
        .from(&owner_address)
        .expect(TxExpect::user_error("str:Cannot call a pool we borrowed from to swap")), 
        |sc| {
            let borrow_wegld = Borrow { 
                token : managed_token_id!(MEX_TOKEN_ID),
                amount: managed_biguint!(9500)
            };
            let mut borrows = MultiValueEncoded::new();
            borrows.push(borrow_wegld);

            let mut actions = MultiValueEncoded::new();
            let action1 = UserAction {
                exchange : Exchange::Xexchange,
                first_token : managed_token_id!(MEX_TOKEN_ID),
                second_token : managed_token_id!(WEGLD_TOKEN_ID),
                amount_swap : managed_biguint!(9500)
            };
            let action2 = UserAction {
                exchange : Exchange::Ashswap,
                first_token : managed_token_id!(second_token),
                second_token : managed_token_id!(first_token),
                amount_swap : managed_biguint!(9400)
            };
            actions.push(action1);
            actions.push(action2);
            sc.borrow(borrows, actions);
        },
        |r| {
            r.assert_user_error("Cannot call a pool we borrowed from to swap")
        }
    );


}

