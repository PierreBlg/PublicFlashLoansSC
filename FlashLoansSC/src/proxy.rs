multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait PairProxyModule {

    #[proxy]
    fn pair_proxy(&self, sc_address: ManagedAddress) -> pair::Proxy<Self::Api>;

}