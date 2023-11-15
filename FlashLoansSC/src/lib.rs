#![no_std]

use pair::ProxyTrait as _;
use types::{Borrow, Exchange, UserAction};

multiversx_sc::imports!();
pub mod storage;
pub mod types;
pub mod deposit;
pub mod actions;
pub mod proxy;
pub mod repay;
pub mod setup;
pub mod fees;



#[multiversx_sc::contract]
pub trait FastLoanContract :
    storage::StorageModule +
    deposit::DepositModule +
    actions::UserActionsModule +
    proxy::PairProxyModule +
    repay::RepayModule +
    setup::SetupModule +
    fees::FeesModule { 


    #[init]
    fn init(
        &self,
        fees : u64,
    ) {
        self.fees_percentage().set(fees);
    }

    #[endpoint(borrow)]
    fn borrow(&self, borrows: MultiValueEncoded<Borrow<Self::Api>>, actions : MultiValueEncoded<UserAction<Self::Api>>) {
        let action_key = self.get_or_create_action_key();
        self.user_actions(action_key).set(actions.to_vec());
        self.length_borrows(action_key).set((borrows.len(), 0usize));
        self.original_caller(action_key).set(self.blockchain().get_caller());

        let vec_borrows = borrows.to_vec();

        let first_element = vec_borrows.get(0);

        // Save all borrows
        self.borrows(action_key).set(vec_borrows);

        // Execute first borrow
        self.execute_borrow(action_key, first_element);
        
    }

    fn execute_borrow(&self, action_key : u64, borrow : Borrow<Self::Api>) {
        let borrow_address_storage = self.token_exchange_map(&Exchange::Xexchange, &borrow.token);
        if borrow_address_storage.is_empty() {
            sc_panic!("Could not execute all borrows")
        }
        let borrow_address = borrow_address_storage.get();

        self.pair_proxy(borrow_address)
        .borrow_liquidity(
            EsdtTokenPayment::new(borrow.token, 0, borrow.amount),
            action_key
        )
        .execute_on_dest_context()
    }


    /* Called by pool after borrow tokens */
    #[payable("*")]
    #[endpoint(depositLoan)]
    fn deposit_loan(
        &self,
        action_key : u64,
    ) {
        // Receive payment and save it 

        let mut payment = self.call_value().single_esdt();
        let pool_caller = self.blockchain().get_caller();

        self.check_caller_is_token_pool_borrow(
            &pool_caller,
            &payment.token_identifier
        );

        // Use this to follow balances
        // Put fees after user actions to calculate user tokens back
        self.save_deposit_tokens(
            action_key,
            payment.clone()
        );

        // Get all borrows and remove current deposit from them

        let borrows_storage = self.borrows(action_key);
        let mut borrows = borrows_storage.get();
        let mut found_borrow = false;
        for (index, borrow) in borrows.iter().enumerate() {
            if borrow.token == payment.token_identifier && borrow.amount == payment.amount {
                found_borrow = true;
                borrows.remove(index);
                break
            }
        }
        require!(found_borrow, "Payment does not correspond to any borrowing asked");
       
        // Add fees to deposit, user will need to give them in addition
        payment.amount = self.add_fee_amount(payment.amount.clone());

        self.save_address_deposit_tokens(
            action_key,
            pool_caller,
            payment.clone()
        );

        let deposit_ended = borrows.len() == 0;

        if deposit_ended {
            let balances = self.execute_user_actions_follow_balance(action_key);
            self.check_and_repay(
                balances,
                action_key
            )
        } else {
            // If it was not the last deposit

            // Get next deposit to get
            let next_borrow = borrows.get(0);

            // Save the remaining deposit list
            borrows_storage.set(borrows);

            // Execute borrow and relaunch this function until no deposits are remaining
            self.execute_borrow(action_key, next_borrow);
        }
    }


    fn check_and_repay(
        &self,
        all_balances : ManagedVec<EsdtTokenPayment>,
        action_key : u64
    ) {
        let give_back_to_user = self.check_actions_result_is_enough(action_key, all_balances);

        self.send_back_to_pools(action_key);

        self.send().direct_multi(&self.original_caller(action_key).get(), &give_back_to_user);
    }
    

}
