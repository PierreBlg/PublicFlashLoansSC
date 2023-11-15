multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub struct BorrowLiquidityContext<M: ManagedTypeApi> {
    pub payment: EsdtTokenPayment<M>,
    pub token_amount_removed : BigUint<M>,
    pub is_first_token : bool,
}

impl<M: ManagedTypeApi> BorrowLiquidityContext<M> {
    pub fn new(
        payment: EsdtTokenPayment<M>,
        is_first_token : bool,
    ) -> Self {
        BorrowLiquidityContext {
            payment,
            token_amount_removed : BigUint::zero(),
            is_first_token
        }
    }
}
