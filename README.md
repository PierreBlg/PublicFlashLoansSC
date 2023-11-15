
# Description step by step

We will go through the contracts step by step, and will describe each function and what is his job in the whole workflow.



## Setup

- Clone the project

- In FastLoanSC, pair and fees-collector folders launch :

```bash
  mxpy contract build
```

# Documentation

The **action_key** of the user is here to keep tracking of the workflow and storages in pool and flashloan contracts. It is unique per call and permit to be certain of the uniqueness of the transaction and workflow.


## Flashloan SC

```rust
#[init]
fn init(
    &self,
    fees : u64,
) {}
```

The init function only takes fees percentage. The fees need to be the same and to be set in both pool and flashloan SC.

```rust
#[only_owner]
#[endpoint(setupPoolsExchanges)]
fn owner_setup_pool_addresses_pairs(&self, pool_addresses : MultiValueEncoded<(Exchange, SwapContractAddress<Self::Api>)>) {}
```

This function is here to save the pool addresses, exchange and token associated to each address. It will avoid too long user input.

```rust
#[endpoint(borrow)]
fn borrow(&self, borrows: MultiValueEncoded<Borrow<Self::Api>>, actions : MultiValueEncoded<UserAction<Self::Api>>) {}
```

The borrow function needs 2 arguments :

- **borrows** which is a list of Token/Amount pairs to borrow :

```rust
pub struct Borrow<M: ManagedTypeApi> {
    pub token : TokenIdentifier<M>,
    pub amount : BigUint<M>,
}
```

- **actions** which is a list of Exchange (1, 2, 3 or 4), first_token, second_token and amount to swap from first to second :

```rust
pub struct UserAction<M: ManagedTypeApi> {
    pub exchange : Exchange,
    pub first_token : TokenIdentifier<M>,
    pub second_token : TokenIdentifier<M>,
    pub amount_swap : BigUint<M>
}

```

All the borrows are made in xExchange pools and we cannot swap in the same pools we borrowed from.

At the end of the borrow function, we call *borrow_liquidity* of first pool :

```rust
self.pair_proxy(borrow_address)
  .borrow_liquidity(
      EsdtTokenPayment::new(borrow.token, 0, borrow.amount),
      action_key
  )
  .execute_on_dest_context()
```

The *borrow_liquidity* function check if everything is fine to borrow (reserves, authorizations, tokens asked, ...) and then save in storage a copy of borrowed token.
Then call *deposit_loan* function in the FlashLoanSC

```rust
self.borrowed_tokens(action_key).update(|f| f.push(
        payment.clone()
    ));
self.flash_loan_proxy(loan_address)
.deposit_loan(action_key)
.with_esdt_transfer(payment)
.execute_on_dest_context()
```

In *deposit_loan* function, we save all deposits per address and in a list. If we need other deposits, we launch again borrow_liquidity (it's a loop) with the new pool and token asked. When the last deposit is made, the workflow continues and executes user actions, by taking address of the actions using their exchange number,tokens and amounts from user input.

```rust 
fn execute_user_actions_follow_balance (
    &self,
    action_key : u64
) -> ManagedVec<EsdtTokenPayment> {}
```

**We keep tracking of the current balances** to check at the end of the workflow if token borrow balances are enough **to pay back pools**.

After user actions, we check balances and if user is able to give back to pools borrowed amounts + additional fees :

```rust 
fn check_actions_result_is_enough(
      &self,
      action_key : u64,
      all_balances : ManagedVec<EsdtTokenPayment>
  ) -> ManagedVec<EsdtTokenPayment> {}
```

So if a user borrows 100 EGLD, with 1% fee, he needs to give back 101 EGLD.

Then the additional amount is sent to the user, the borrowed + fees amount is sent to pools

Pools check again using their saved borrowed token if everything is ok, and then pools send fees to collector.

At the complete end, after pay back, stack goes back in pools, in *borrow_liquidity* function, after each deposit to flashloan, and check that the liquidity is the same as before execution.

The workflow is the following :

```
1. Borrow tokens
2. Ask pools to deposit in deposit_loan 
3. Pool save borrows
3. Wait for all deposits in FlSC
4. Executes user actions and track balances
5. Check if user can pay back amounts + fees to pools 
6. Check amount the user will receive
7. Send back amounts + fees to pool 
8. Pools send extra fees to collector
9. User receive his extra amount of tokens
10. Each pool check in borrow_liquidity that liquidity is the same as before
```


## Running Tests

To be certain everything is working as expected, whiteboxes tests were made. 

To test the smart contract, go in FastLoanSC folder and launch :

```bash
  cargo test
```


## Tests

**try_setup** - Setup the contracts

**borrow_to_not_set_exchange_pool** - Borrow an imaginary token

**fastloan_deposit_from_unauthorized** - Call deposit_loan from unauthorized address

**pool_borrow_incorrect_token** - FastloanSC tries to borrow incorrect tokens from pool

**pool_endpoints_unauthorized** - Force calling endpoint with unauthorized address

**borrow_more_than_pool_reserve** - Borrow more than pool has tokens

**user_action_to_unknown_contract** - User action is directed to an unknown contract

**borrow_give_back_no_fee** - Borrow 100 tokens and give them back (no fees)

**borrow_two_tokens_give_back_no_fee** - Borrow 100 of two tokens and give them back (no fees)

**arbitrage_between_two_differents_exchanges_pools_fees** - Do arbitrage between 2 pools of 2 differents exchanges, and test swap and borrow same pool

**borrow_and_swap_other_pool_swap_back** - Swap EGLD to ASH and ASH to WEGLD using the same pool. Cannot pay fees.

**pool_borrow_and_swap_same_pool** - Try to borrow and to swap in the same pool to check if price imbalanced, and then swap in another exchange to give back to pool -> Price is calculated again between deposit and give back funds due to storage drop before deposit, cannot exploit borrowing