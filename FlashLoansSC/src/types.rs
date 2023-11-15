use common_structs::TokenPair;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();


#[derive(
    TypeAbi,
    NestedEncode,
    NestedDecode,
)]
pub struct SwapContractAddress<M: ManagedTypeApi> {
    pub address : ManagedAddress<M>,
    pub tokens : TokenPair<M>
}

#[derive(
    ManagedVecItem,
    TopDecode,
    TypeAbi,
    NestedEncode,
    NestedDecode,
)]
pub enum Exchange {
    Xexchange,
    Jexchange,
    Ashswap,
    JewelSwap
}

#[derive(
    TopEncode,
    TopDecode,
    TypeAbi,
    NestedEncode,
    NestedDecode,
    ManagedVecItem,
    Clone
)]
pub struct Borrow<M: ManagedTypeApi> {
    pub token : TokenIdentifier<M>,
    pub amount : BigUint<M>,
}


#[derive(
    ManagedVecItem,
    TopEncode,
    TopDecode,
    TypeAbi,
    NestedEncode,
    NestedDecode,
)]
pub struct UserAction<M: ManagedTypeApi> {
    pub exchange : Exchange,
    pub first_token : TokenIdentifier<M>,
    pub second_token : TokenIdentifier<M>,
    pub amount_swap : BigUint<M>
}
