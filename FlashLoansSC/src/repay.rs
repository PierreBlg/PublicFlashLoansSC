use crate::{storage, proxy, fees};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait RepayModule :
    storage::StorageModule +
    proxy::PairProxyModule +
    fees::FeesModule {

    fn check_actions_result_is_enough(
        &self,
        action_key : u64,
        all_balances : ManagedVec<EsdtTokenPayment>
    ) -> ManagedVec<EsdtTokenPayment> {
        let all_deposits = self.deposits(action_key).get();

        let mut give_back_to_user : ManagedVec<EsdtTokenPayment> = ManagedVec::new();

        for balance in all_balances.into_iter() {
            
            let mut found = false;
            for deposit in all_deposits.into_iter() {
                    if deposit.token_identifier == balance.token_identifier {
                        let deposit_with_fees = self.add_fee_amount(deposit.amount);
                        if balance.amount < deposit_with_fees {
                            sc_panic!("One of the borrowed token amount was not entirely given back")
                        } else {
                            if balance.amount > deposit_with_fees {
                                give_back_to_user.push(
                                    EsdtTokenPayment::new(
                                        balance.token_identifier.clone(),
                                        0,
                                        &balance.amount - &deposit_with_fees
                                    )
                                )
                            }
                        }
                        found = true;
                    }
            }

            if !found &&  balance.amount > BigUint::zero() {
                give_back_to_user.push(balance)
            }
        }

        give_back_to_user
    }

    fn send_back_to_pools(&self, action_key : u64) {
        let all_deposits_from_address_storage = self.deposits_per_address(action_key);

        for (address, payments) in all_deposits_from_address_storage.iter() {
            let _ : IgnoreValue = pair::flashloan::ProxyTrait::repay(
                &mut self.pair_proxy(address),
                action_key
            )
            .with_multi_token_transfer(payments)
            .execute_on_dest_context();
        }
    }

}