use crate::{contexts::base::StorageCache, config, errors::{ERROR_ONE_BORROW_NOT_GIVEN_BACK, ERROR_TOO_MUCH_TOKENS_GIVEN_BACK, ERROR_NOT_ENTIRELY_GIVEN_BACK, ERROR_CALLER_IS_NOT_FLASH_LOAN_SC, ERROR_NO_FLASH_LOAN_SC_SET}, fee, amm, liquidity_pool};
pub const MAX_PERCENTAGE: u64 = 10_000;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod flash_loan_sc {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait FlashLoanContract {
        #[payable("*")]
        #[endpoint(depositLoan)]
        fn deposit_loan(&self, action_key: u64);
    }
}



#[multiversx_sc::module]
pub trait FlashLoanModule :
    config::ConfigModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
    + fee::FeeModule 
    + liquidity_pool::LiquidityPoolModule
    + amm::AmmModule
    {

    #[proxy]
    fn flash_loan_proxy(&self, sc_address: ManagedAddress) -> flash_loan_sc::Proxy<Self::Api>;

    fn call_deposit_loan(
        &self,
        loan_address: ManagedAddress,
        payment : EsdtTokenPayment,
        action_key : u64,
        is_first_token : bool,
        specific_token_reserve_before_borrow : BigUint,
        lp_supply_before_borrow : BigUint
    ) {
        self.borrowed_tokens(action_key).update(|f| f.push(
            payment.clone()
        ));

        let _ : IgnoreValue = self.flash_loan_proxy(loan_address)
        .deposit_loan(action_key)
        .with_esdt_transfer(payment)
        .execute_on_dest_context();

        // Check if liquidity is ok for each token (max 2 for this pool)
        // Check invariants

        let storage_cache = StorageCache::new(self);

        if is_first_token {
            require!(
                storage_cache.first_token_reserve == specific_token_reserve_before_borrow, 
                "Liquidity is incorrect after execution"
            );
        } else {
            require!(
                storage_cache.second_token_reserve == specific_token_reserve_before_borrow, 
                "Liquidity is incorrect after execution"
            );
        };

        require!(
            lp_supply_before_borrow == storage_cache.lp_token_supply,
            "LP supply is incorrect after execution"
        )
    }

    #[payable("*")]
    #[endpoint(repay)]
    fn repay(&self, action_key : u64) {

        self.check_loan_caller_and_return();

        let payments = self.call_value().all_esdt_transfers();
        let borrowed_tokens = self.borrowed_tokens(action_key).get();
        let mut storage_cache = StorageCache::new(self);
        let mut fees : ManagedVec<EsdtTokenPayment> = ManagedVec::new();

        for borrow in borrowed_tokens.into_iter() {
            let mut found = false;
            for payment in payments.into_iter() {
                if borrow.token_identifier == payment.token_identifier {
                    let borrow_with_fees = self.add_fee_amount(borrow.amount.clone());
                    if &payment.amount < &borrow_with_fees {
                        sc_panic!(ERROR_NOT_ENTIRELY_GIVEN_BACK)
                    } else if &payment.amount == &borrow_with_fees {
                        if borrow.token_identifier == storage_cache.first_token_id {
                            storage_cache.first_token_reserve += borrow.amount.clone();
                        } else if borrow.token_identifier == storage_cache.second_token_id  {
                            storage_cache.second_token_reserve += borrow.amount.clone();
                        }
                        fees.push(EsdtTokenPayment::new(borrow.token_identifier.clone(), 0, &borrow_with_fees - &borrow.amount))
                    } else {
                        sc_panic!(ERROR_TOO_MUCH_TOKENS_GIVEN_BACK)
                    }
                    found = true
                }
            }
            if !found {
                sc_panic!(ERROR_ONE_BORROW_NOT_GIVEN_BACK)
            }
        }

        // Fees sent to fees collector
        for fee in fees.iter() {
            self.send_fees_collector_cut(fee.token_identifier, fee.amount);
        };

    }

    fn check_loan_caller_and_return(&self) -> ManagedAddress {
        let loan_address_storage = self.flash_loan_sc();

        require!(
            !loan_address_storage.is_empty(),
            ERROR_NO_FLASH_LOAN_SC_SET
        );

        let loan_address = loan_address_storage.get();

        require!(
            self.blockchain().get_caller() == loan_address,
            ERROR_CALLER_IS_NOT_FLASH_LOAN_SC
        );

        loan_address
    }
    
    fn add_fee_amount(&self, payment_amount: BigUint) -> BigUint {
        let fee_percentage = self.flashloan_fees_percent().get();
        let fee_amount = &payment_amount * fee_percentage / MAX_PERCENTAGE;
        payment_amount + fee_amount
    }

    #[view(borrowedTokens)]
    #[storage_mapper("borrowed_tokens")]
    fn borrowed_tokens(&self, action_key : u64) -> SingleValueMapper<ManagedVec<EsdtTokenPayment>>;

}