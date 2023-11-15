use crate::{storage, types::Exchange};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait DepositModule :
    storage::StorageModule {

    fn save_address_deposit_tokens(
        &self,
        action_key : u64,
        pool_caller : ManagedAddress,
        deposit_payment : EsdtTokenPayment,
    ) {
        let mut deposits_per_address_storage = self.deposits_per_address(action_key);
        
        let deposits_for_caller = deposits_per_address_storage.get(&pool_caller);
        if let Some(mut already_deposit) = deposits_for_caller {
            already_deposit.push(deposit_payment.clone());
            deposits_per_address_storage.insert(pool_caller, already_deposit);
        } else {
            let mut new_deposits_for_address = ManagedVec::new();
            new_deposits_for_address.push(deposit_payment.clone());
            deposits_per_address_storage.insert(pool_caller, new_deposits_for_address);
        }
    }

    fn save_deposit_tokens(
        &self,
        action_key : u64,
        deposit_payment : EsdtTokenPayment
    ) {
        let deposits_storage = self.deposits(action_key);
        if deposits_storage.is_empty() {
            let mut all_deposits = ManagedVec::new();
            all_deposits.push(deposit_payment);
            deposits_storage.set(all_deposits);
        } else {
            deposits_storage.update(|f| f.push(deposit_payment));
        }
    }

    fn check_caller_is_token_pool_borrow(
        &self,
        pool_caller : &ManagedAddress,
        token_received : &TokenIdentifier
    ) {
        let caller_token_borrow_storage = self.token_exchange_map(&Exchange::Xexchange, token_received);
        if caller_token_borrow_storage.is_empty() {
            sc_panic!("Caller address is not defined")
        }
        if pool_caller != &caller_token_borrow_storage.get() {
            sc_panic!("Caller address is not defined")
        }
    }
    

}