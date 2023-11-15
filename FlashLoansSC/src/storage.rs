use crate::types::{UserAction, Exchange, Borrow};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait StorageModule {

    // Setup Storage

    #[view(token_exchange_map)]
    #[storage_mapper("token_exchange_map")]
    fn token_exchange_map(&self, exchange : &Exchange, token : &TokenIdentifier) -> SingleValueMapper<ManagedAddress>;

    #[view(token_exchange_pair_map)]
    #[storage_mapper("token_exchange_pair_map")]
    fn token_exchange_pair_map(&self, exchange : &Exchange, first_token : &TokenIdentifier, second_token : &TokenIdentifier) -> SingleValueMapper<ManagedAddress>;

    // Repay Storage

    #[storage_mapper("original_caller")]
    fn original_caller(&self, action_key : u64) -> SingleValueMapper<ManagedAddress>;

    // Deposit Storage

    #[storage_mapper("borrows")]
    fn borrows(&self, action_key : u64) -> SingleValueMapper<ManagedVec<Borrow<Self::Api>>>;

    #[storage_mapper("length_borrows")]
    fn length_borrows(&self, action_key : u64) -> SingleValueMapper<(usize, usize)>;

    #[storage_mapper("deposits_per_address")]
    fn deposits_per_address(&self, action_key : u64) -> MapMapper<ManagedAddress, ManagedVec<EsdtTokenPayment>>;

    #[storage_mapper("deposits")]
    fn deposits(&self, action_key : u64) -> SingleValueMapper<ManagedVec<EsdtTokenPayment>>;

    // Action Storage

    #[view(user_actions)]
    #[storage_mapper("user_actions")]
    fn user_actions(&self, action_key : u64) -> SingleValueMapper<ManagedVec<UserAction<Self::Api>>>;

    #[storage_mapper("action_key")]
    fn action_key(&self) -> SingleValueMapper<u64>;

    fn get_or_create_action_key(&self) -> u64 {
        let key_mapper = self.action_key();
        if key_mapper.is_empty() {
            key_mapper.set(1u64);
            1u64
        } else {
            let new_key = key_mapper.get() + 1u64;
            key_mapper.set(new_key);
            new_key
        }
    }
}