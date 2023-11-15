use crate::{storage, types::{Exchange, SwapContractAddress}};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait SetupModule :
    storage::StorageModule {

    #[only_owner]
    #[endpoint(setupPoolsExchanges)]
    fn owner_setup_pool_addresses_pairs(&self, pool_addresses : MultiValueEncoded<(Exchange, SwapContractAddress<Self::Api>)>) {
        for (exchange, swap_contract) in pool_addresses.into_iter() {
            let first_token_storage = self.token_exchange_map(&exchange, &swap_contract.tokens.first_token);
            let second_token_storage = self.token_exchange_map(&exchange, &swap_contract.tokens.second_token);
            let pair_token_storage = self.token_exchange_pair_map(&exchange, &swap_contract.tokens.first_token, &swap_contract.tokens.second_token);

            first_token_storage.set(swap_contract.address.clone());
            second_token_storage.set(swap_contract.address.clone());
            pair_token_storage.set(swap_contract.address.clone());
            
        }
    }

}