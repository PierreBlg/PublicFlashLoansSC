use pair::ProxyTrait as _;

use crate::{storage, proxy, types::Exchange};

multiversx_sc::imports!();


#[multiversx_sc::module]
pub trait UserActionsModule :
    storage::StorageModule +
    proxy::PairProxyModule {

    fn execute_user_actions_follow_balance (
        &self,
        action_key : u64
    ) -> ManagedVec<EsdtTokenPayment> {
        let actions = self.user_actions(action_key).get();
        let mut balances =  self.deposits(action_key).get().clone();
        let addresses = self.deposits_per_address(action_key);

        for action in actions.into_iter() {    
            let contract = self.get_contract_address_for_action(
                &action.exchange,
                &action.first_token,
                &action.second_token
            );

            if addresses.contains_key(&contract) {
                sc_panic!("Cannot call a pool we borrowed from to swap")
            }

            let result : EsdtTokenPayment = self
            .pair_proxy(contract)
            .swap_tokens_fixed_input(action.second_token, 1u64)
            .with_esdt_transfer(EsdtTokenPayment::new(action.first_token.clone(), 0, action.amount_swap.clone()))
            .execute_on_dest_context();


            /* Follow balance of each token after each round */
            let mut found = false;
            for mut balance in balances.iter() {

                /* If token swap already followed, add amount to it */
                if balance.token_identifier == result.token_identifier {
                    balance.amount += result.amount.clone();
                    found = true;
                }

                /* Remove amount of token origin */
                if balance.token_identifier == action.first_token {
                    balance.amount -= action.amount_swap.clone();
                }
            }
            
            /* If token swap not already followed, add EsdtTokenPayment to followed */
            if !found {
                balances.push(EsdtTokenPayment::new(
                    result.token_identifier.clone(),
                    0,
                    result.amount,
                ));
            }
        }

        balances
    }

    fn get_contract_address_for_action(
        &self,
        exchange : &Exchange,
        first_token : &TokenIdentifier,
        second_token : &TokenIdentifier
    ) -> ManagedAddress {

        let contract;
        let contract_storage = self.token_exchange_pair_map(exchange, first_token, second_token);
        if contract_storage.is_empty() {
            let contract_reverse = self.token_exchange_pair_map(exchange, second_token, first_token);
            if contract_reverse.is_empty() {
                sc_panic!("A swap pair is unavailable for the action selected")
            } else {
                contract = contract_reverse.get();
            }
        } else {
            contract = contract_storage.get();
        }

        contract
    }
}