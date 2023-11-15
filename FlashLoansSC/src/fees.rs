multiversx_sc::imports!();

pub const MAX_PERCENTAGE: u64 = 10_000;

#[multiversx_sc::module]
pub trait FeesModule {

    #[storage_mapper("fees_percentage")]
    fn fees_percentage(&self) -> SingleValueMapper<u64>;
    
    fn add_fee_amount(&self, payment_amount: BigUint) -> BigUint {
        let fee_percentage = self.fees_percentage().get();
        let fee_amount = &payment_amount * fee_percentage / MAX_PERCENTAGE;
        payment_amount + fee_amount
    }

}