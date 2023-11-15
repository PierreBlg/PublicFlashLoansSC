use common_structs::TokenPair;
use multiversx_sc::types::{MultiValueEncoded, ManagedAddress, Address};
use multiversx_sc_scenario::{*, scenario_model::{SetStateStep, Account, ScDeployStep, AddressValue, ScCallStep}};
use fastloan::{self, FastLoanContract, types::{Exchange, SwapContractAddress}, setup::SetupModule};
use pair::{Pair, config::ConfigModule, fee::FeeModule};
use fees_collector::{FeesCollector, config::ConfigModule as FeesCollectorConfigModule};
use pausable::{State, PausableModule};

const FASTLOAN_PATH_EXPR: &str = "file:output/fastloan.wasm";
pub const PAIR_PATH_EXPR: &str = "file:../xExchange/pair/output/pair.wasm";
pub const FEES_COLLECTOR_PATH_EXPR: &str = "file:../xExchange/fees-collector/output/fees-collector.wasm";

pub const WEGLD_TOKEN_ID : &[u8] = b"WEGLD-abcdef";
pub const MEX_TOKEN_ID : &[u8] = b"MEX-abcdef";
const LP_TOKEN_ID : &[u8] = b"LPTOK-abcdef";
const LOCKED_TOKEN_ID : &[u8] = b"LKTKN-abcdef";

pub const OWNER_ADDRESS_EXPR: &str = "address:owner";
pub const FEES_COLLECTOR_OWNER: &str = "address:owner-fees";
pub const POOL_ADDRESS_EXPR: &str = "sc:pool";
pub const FASTLOAN_ADDRESS_EXPR: &str = "sc:fastloan";
pub const FEES_COLLECTOR_ADDRESS_EXPR : &str = "sc:fees_collector";


pub fn setup(flashloan_fees : u64) -> (ScenarioWorld, WhiteboxContract<pair::ContractObj<DebugApi>>, WhiteboxContract<fastloan::ContractObj<DebugApi>>, Address, WhiteboxContract<fees_collector::ContractObj<DebugApi>>) {
    let mut world = ScenarioWorld::new();

    world.register_contract(PAIR_PATH_EXPR, pair::ContractBuilder);
    world.register_contract(FEES_COLLECTOR_PATH_EXPR, fees_collector::ContractBuilder);

    let fees_collector_whitebox = create_fee_collector(&mut world);

    let pool_whitebox = create_pool(
        &mut world,
        POOL_ADDRESS_EXPR,
        OWNER_ADDRESS_EXPR,
        WEGLD_TOKEN_ID,
        MEX_TOKEN_ID,
        LP_TOKEN_ID,
        flashloan_fees
    );

    let owner_address = AddressValue::from(OWNER_ADDRESS_EXPR).to_address();
    let fastloan_whitebox = WhiteboxContract::new(FASTLOAN_ADDRESS_EXPR, fastloan::contract_obj);
    let fastloan_code = world.code_expression(FASTLOAN_PATH_EXPR);
    world.register_contract(FASTLOAN_PATH_EXPR, fastloan::ContractBuilder);


    world.set_state_step(
        SetStateStep::new()
        .new_address(OWNER_ADDRESS_EXPR, 2, FASTLOAN_ADDRESS_EXPR)
    );

    world.whitebox_deploy(
        &fastloan_whitebox,
        ScDeployStep::new()
        .from(OWNER_ADDRESS_EXPR)
        .code(fastloan_code),
        |sc| {
            let mut multi_value = MultiValueEncoded::new();
            let pair = TokenPair {
                first_token : managed_token_id!(WEGLD_TOKEN_ID),
                second_token : managed_token_id!(MEX_TOKEN_ID)
            };
            let pool = (Exchange::Xexchange, SwapContractAddress {
                address : managed_address!(&AddressValue::from(POOL_ADDRESS_EXPR).to_address()),
                tokens: pair
            });
            multi_value.push(pool);
            sc.init(
                flashloan_fees
            );
            sc.owner_setup_pool_addresses_pairs(multi_value);
        }
    )
    .whitebox_call(
        &pool_whitebox,
        ScCallStep::new()
        .from(OWNER_ADDRESS_EXPR)
        .to(POOL_ADDRESS_EXPR)
        .esdt_transfer(WEGLD_TOKEN_ID, 0, rust_biguint!(10000))
        .esdt_transfer(MEX_TOKEN_ID, 0, rust_biguint!(10000))
        .no_expect(), 
        |sc| {
            sc.add_liquidity(rust_biguint!(10000).into(), rust_biguint!(10000).into());
            sc.set_flash_loan_sc(managed_address!(&AddressValue::from(FASTLOAN_ADDRESS_EXPR).to_address()));
        }
    );

    world.whitebox_call(
        &fees_collector_whitebox,
        ScCallStep::new()
        .from(OWNER_ADDRESS_EXPR)
        .no_expect(), 
        |sc| {
            let mut known_contracts =  MultiValueEncoded::new();
            let mut known_tokens =  MultiValueEncoded::new();
            known_contracts.push(managed_address!(&AddressValue::from(POOL_ADDRESS_EXPR).to_address()));
            known_tokens.push(managed_token_id!(WEGLD_TOKEN_ID));
            known_tokens.push(managed_token_id!(MEX_TOKEN_ID));
            sc.add_known_contracts(known_contracts);
            sc.add_known_tokens(known_tokens)
        }
    );

    world.whitebox_call(
        &pool_whitebox,
        ScCallStep::new()
        .from(OWNER_ADDRESS_EXPR), 
        |sc| {
            sc.setup_fees_collector(
                managed_address!(&AddressValue::from(FEES_COLLECTOR_ADDRESS_EXPR).to_address()),
                1u64
            );
        }
    );
    

    return (
        world,
        pool_whitebox,
        fastloan_whitebox,
        owner_address,
        fees_collector_whitebox
    )
    
       
}

pub fn create_pool(
    world : &mut ScenarioWorld,
    pool_expr : &str,
    owner_address_expr : &str,
    first_token : &[u8],
    second_token: &[u8],
    lp_token : &[u8],
    fee_percentage : u64,
) -> WhiteboxContract<pair::ContractObj<DebugApi>> {
      let pool_whitebox = WhiteboxContract::new(pool_expr, pair::contract_obj);
    let pool_code = world.code_expression(PAIR_PATH_EXPR);

    world.set_state_step(
        SetStateStep::new()
        .put_account(owner_address_expr, 
        Account::new()
        .nonce(1)
        .esdt_balance(first_token.to_vec(), rust_biguint!(10000000000))
        .esdt_balance(second_token.to_vec(), rust_biguint!(10000000000))
    )
    );

    let owner_address = AddressValue::from(owner_address_expr).to_address();

    let roles = vec![
        "ESDTRoleLocalMint".to_string(),
        "ESDTRoleLocalBurn".to_string(),
    ];

    world.set_state_step(
        SetStateStep::new()
        .put_account(pool_expr, Account::new()
        .esdt_roles(lp_token.to_vec(), roles) 
        .code(pool_code.clone())
    ));

    world.whitebox_call(
        &pool_whitebox, 
        ScCallStep::new()
        .from(owner_address_expr),
        |sc| {
            sc.init(
                managed_token_id!(first_token),
                managed_token_id!(second_token),
                managed_address!(&owner_address),
                managed_address!(&owner_address),
                300u64,
                50u64,
                fee_percentage,
                ManagedAddress::<DebugApi>::zero(),
                MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new()
            );
            sc.state().set(State::Active);
            sc.set_lp_token_identifier(managed_token_id!(lp_token));
        }
    );


    pool_whitebox

    
}


pub fn create_fee_collector(
    world : &mut ScenarioWorld
) -> WhiteboxContract<fees_collector::ContractObj<DebugApi>> {
    let fees_collector_whitebox = WhiteboxContract::new(FEES_COLLECTOR_ADDRESS_EXPR, fees_collector::contract_obj);
    let fee_collector_code = world.code_expression(FEES_COLLECTOR_PATH_EXPR);

    world.set_state_step(
        SetStateStep::new()
        .put_account(FEES_COLLECTOR_OWNER, 
        Account::new()
        .nonce(1)
    )
    );

    world.set_state_step(
        SetStateStep::new()
        .put_account(FEES_COLLECTOR_ADDRESS_EXPR, Account::new()
        .code(fee_collector_code.clone())
    ));

    world.whitebox_call(
        &fees_collector_whitebox, 
        ScCallStep::new()
        .from(FEES_COLLECTOR_OWNER),
        |sc| {
            sc.init(
                managed_token_id!(LOCKED_TOKEN_ID),
                managed_address!(&AddressValue::from("sc:energy").to_address())
            );
        }
    );

    fees_collector_whitebox

}