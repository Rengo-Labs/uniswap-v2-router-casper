use casper_contract::{ contract_api::{runtime}};
use casper_engine_test_support::AccountHash;
use casper_types::{U256, Key, runtime_args, RuntimeArgs, contracts::{ContractHash}};
use test_env::{Sender, TestEnv, TestContract};

use crate::uniswap_instance::UniswapInstance;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;

const NAME: &str = "uniswap_router";

fn deploy_dummy_tokens(env: &TestEnv, owner: Option<AccountHash>) -> (TestContract, TestContract, TestContract) 
{
    let decimals: u8 = 18;
    let init_total_supply: U256 = 1000.into();

    let token1_owner = if owner.is_none() { env.next_user() } else { owner.unwrap()};
    let token1_contract = TestContract::new(
        &env,
        "token.wasm",
        "token1_contract",
        Sender(token1_owner),
        runtime_args! {
            "initial_supply" => init_total_supply,
            "name" => "token1",
            "symbol" => "tk1",
            "decimals" => decimals
        }
    );

    let token2_owner = if owner.is_none() { env.next_user() } else { owner.unwrap()};
    let token2_contract = TestContract::new(
        &env,
        "token.wasm",
        "token2_contract",
        Sender(token2_owner),
        runtime_args! {
            "initial_supply" => init_total_supply,
            "name" => "token2",
            "symbol" => "tk2",
            "decimals" => decimals
        }
    );

    let token3_owner = if owner.is_none() { env.next_user() } else { owner.unwrap()};
    let token3_contract = TestContract::new(
        &env,
        "token.wasm",
        "token3_contract",
        Sender(token3_owner),
        runtime_args! {
            "initial_supply" => init_total_supply,
            "name" => "token3",
            "symbol" => "tk3",
            "decimals" => decimals
        }
    );
    (token1_contract, token2_contract, token3_contract)
}

fn deploy_uniswap_router() -> (TestEnv, UniswapInstance, AccountHash, TestContract, TestContract, TestContract) 
{
    let env = TestEnv::new();
    let owner = env.next_user();

    // deploy factory contract
    let env_factory = TestEnv::new();
    let owner_factory = env.next_user();
    let factory_contract = TestContract::new(
        //&env_factory,
        &env,
        "factory.wasm",
        "factory",
        Sender(owner_factory),
        runtime_args! {
            "fee_to_setter" => Key::from(owner_factory)
            // contract_name is passed seperately, so we don't need to pass it here.
        }
    );
    
    // deploy wcspr contract
    let env_wcspr = TestEnv::new();
    let owner_wcspr = env_wcspr.next_user();
    let wcspr = TestContract::new(
        // &env_wcspr,
        &env,
        "wcspr.wasm",
        "wcspr",
        Sender(owner_wcspr),
        //Sender(owner),
        runtime_args! {}
    );

    // deploy library contract
    let env_library = TestEnv::new();
    let owner_library = env_library.next_user();
    let library_contract = TestContract::new(
        // &env_library,
        &env,
        "library.wasm",
        "library",
        Sender(owner_library),
        runtime_args! {}
    );
    
    // deploy pair contract
    let owner_pair = env_library.next_user();
    let pair_contract = TestContract::new(
        // &env_library,
        &env,
        "pair.wasm",
        "pair",
        //Sender(owner_pair),
        Sender(owner),
        runtime_args! {
            "callee_contract_hash" => Key::from(owner),
            "factory_hash" => Key::Hash(factory_contract.contract_hash()),
        }
    );

    let (token1, token2, token3) = deploy_dummy_tokens(&env, Some(owner));             // deploy dummy tokens for pair initialize

    
    let args: RuntimeArgs = runtime_args!{
        "token0" => Key::Hash(token1.contract_hash()),
        "token1" => Key::Hash(token2.contract_hash()),
        "factory_hash" => Key::Hash(factory_contract.contract_hash())
    };
    pair_contract.call_contract(Sender(owner), "initialize", args);
    

    let token = UniswapInstance::new(
        &env,
        NAME,
        Key::Hash(factory_contract.contract_hash()),
        Key::Hash(wcspr.contract_hash()),
        Key::Hash(library_contract.contract_hash()),
        Key::Hash(pair_contract.contract_hash()),
       // Key::Hash(token_a.contract_hash()),
       // Key::Hash(token_b.contract_hash()),
        Sender(owner)
    );
    
    //Approve Uniswap to spend wcspr on owner_wcspr's behalf                
    //token.approve(&wcspr, Sender(owner), token.uniswap_contract_package_hash(), 1000.into());
    
    
    // Approve router to transfer to pair
    let args: RuntimeArgs = runtime_args!{
        "spender" => token.uniswap_contract_package_hash(),
        "amount" => U256::from(1000)
    };
    pair_contract.call_contract(Sender(owner), "approve", args);
    
    (env, token, owner, token1, token2, token3)
}


#[test]
fn test_uniswap_deploy()
{
    let (env, token, owner, _, _, _) = deploy_uniswap_router();
    println!("{}", owner);
    let self_hash: Key = token.uniswap_contract_address();
    let pair_hash: ContractHash = token.uniswap_pair_address();
    let package_hash: Key = token.uniswap_contract_package_hash();

    let zero_addr:Key = Key::from_formatted_str("hash-0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    assert_ne!(self_hash, zero_addr);
    assert_ne!(Key::from(pair_hash), zero_addr);                // pair successfully deployed
    assert_ne!(package_hash, zero_addr);
}


#[test]
fn add_liquidity()                                              // Working
{
    let (env, uniswap, owner, token1, token2, token3) = deploy_uniswap_router();

    let token_a = Key::Hash(token1.contract_hash());
    let token_b = Key::Hash(token2.contract_hash());
    let to = Key::Hash(token3.contract_hash());
    
    let mut rng = rand::thread_rng();
    let amount_a_desired: U256 = rng.gen_range(300..600).into();
    let amount_b_desired: U256 = rng.gen_range(300..600).into();
    let amount_a_min: U256 = rng.gen_range(1..250).into();
    let amount_b_min: U256 = rng.gen_range(1..250).into();

    println!("\nadd_liquidity");
    println!("\namount_a_desired: {}", amount_a_desired);
    println!("\namount_b_desired: {}", amount_b_desired);
    println!("\namount_a_min: {}", amount_a_min);
    println!("\nmount_b_min: {}\n", amount_b_min);
    
/*    
    let amount_a_desired: U256 = 557.into();
    let amount_b_desired: U256 = 393.into();
    let amount_a_min: U256 = 132.into();
    let amount_b_min: U256 = 70.into();
*/
    let deadline: u128 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_millis() + (1000 * (30 * 60)),      // current epoch time in milisecond + 30 minutes
        Err(_) => 0
    };

    // approve the router to spend tokens
    uniswap.approve(&token1, Sender(owner), uniswap.uniswap_contract_package_hash(), amount_a_desired);
    uniswap.approve(&token2, Sender(owner), uniswap.uniswap_contract_package_hash(), amount_b_desired);
    
    assert_eq!(uniswap.allowance(&token1, owner, uniswap.uniswap_contract_package_hash()), amount_a_desired);             // Token approved
    assert_eq!(uniswap.allowance(&token2, owner, uniswap.uniswap_contract_package_hash()), amount_b_desired);             // Token approved

    uniswap.add_liquidity(Sender(owner), token_a, token_b, amount_a_desired, amount_b_desired, amount_a_min, amount_b_min, to, deadline.into());
}


#[test]
fn add_liquidity_cspr()                                     // Working
{
    let (env, uniswap, owner, token1, token2, _) = deploy_uniswap_router();

    let to = Key::Hash(token2.contract_hash());

    let mut rng = rand::thread_rng();
    let token = Key::Hash(token1.contract_hash());
    let amount_token_desired: U256 = rng.gen_range(300..600).into();
    let amount_cspr_desired: U256 = rng.gen_range(300..600).into();
    let amount_token_min: U256 = rng.gen_range(1..250).into();
    let amount_cspr_min: U256 = rng.gen_range(1..250).into();

    println!("\nadd_liquidity_cspr");
    println!("\namount_token_desired: {}", amount_token_desired);
    println!("\namount_cspr_desired: {}", amount_cspr_desired);
    println!("\namount_token_min: {}", amount_token_min);
    println!("\namount_cspr_min: {}\n", amount_cspr_min);

    let deadline: u128 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_millis() + (1000 * (30 * 60)),      // current epoch time in milisecond + 30 minutes
        Err(_) => 0
    };

    uniswap.approve(&token1, Sender(owner), uniswap.uniswap_contract_package_hash(), amount_token_desired);
    uniswap.add_liquidity_cspr(Sender(owner), token, amount_token_desired, amount_cspr_desired, amount_token_min, amount_cspr_min, to, deadline.into());
}


//#[test]
fn remove_liquidity()                                           // Working
{
    let (env, uniswap, owner, token1, token2, token3) = deploy_uniswap_router();
    //let (token1, token2, token3) = deploy_dummy_tokens(&env, None);
    let mut rng = rand::thread_rng();
    

    // First Add liquidity
    let token_a = Key::Hash(token1.contract_hash());
    let token_b = Key::Hash(token2.contract_hash());
    let to = Key::Hash(token3.contract_hash());

    let amount_a_desired: U256 = rng.gen_range(300..600).into();
    let amount_b_desired: U256 = rng.gen_range(300..600).into();
    let amount_a_min: U256 = rng.gen_range(1..250).into();
    let amount_b_min: U256 = rng.gen_range(1..250).into();
    
    let deadline: u128 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_millis() + (1000 * (30 * 60)),      // current epoch time in milisecond + 30 minutes
        Err(_) => 0
    };

    // approve the router to spend tokens
    uniswap.approve(&token1, Sender(owner), uniswap.uniswap_contract_package_hash(), amount_a_desired);
    uniswap.approve(&token2, Sender(owner), uniswap.uniswap_contract_package_hash(), amount_b_desired);
    
    assert_eq!(uniswap.allowance(&token1, owner, uniswap.uniswap_contract_package_hash()), amount_a_desired);             // Token approved
    assert_eq!(uniswap.allowance(&token2, owner, uniswap.uniswap_contract_package_hash()), amount_b_desired);             // Token approved

    uniswap.add_liquidity(Sender(owner), token_a, token_b, amount_a_desired, amount_b_desired, amount_a_min, amount_b_min, to, deadline.into());


    // Now remove liquidity
    let token_a = Key::Hash(token1.contract_hash());
    let token_b = Key::Hash(token2.contract_hash());
    let liquidity:U256 = rng.gen_range(300..500).into();
    let amount_a_min:U256 = rng.gen_range(1..250).into();
    let amount_b_min:U256 = rng.gen_range(1..250).into();
    let to = Key::Hash(token3.contract_hash());

    println!("\nremove_liquidity");
    println!("\nliquidity: {}", liquidity);
    println!("\namount_a_min: {}", amount_a_min);
    println!("\namount_b_min: {}", amount_b_min);

    uniswap.remove_liquidity(Sender(owner), token_a, token_b, liquidity, amount_a_min, amount_b_min, to, deadline.into());
}


#[test]
fn remove_liquidity_cspr()
{
    let (env, uniswap, owner, token1, token2, _) = deploy_uniswap_router();
    let mut rng = rand::thread_rng();

    let token = Key::Hash(token1.contract_hash());
    let amount_token_desired: U256 = rng.gen_range(300..600).into();
    let amount_cspr_desired: U256 = rng.gen_range(300..600).into();
    let amount_token_min: U256 = rng.gen_range(1..250).into();
    let amount_cspr_min: U256 = rng.gen_range(1..250).into();
    let to = Key::Hash(token2.contract_hash());

    let deadline: u128 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_millis() + (1000 * (30 * 60)),      // current epoch time in milisecond + 30 minutes
        Err(_) => 0
    };

    uniswap.approve(&token1, Sender(owner), uniswap.uniswap_contract_package_hash(), amount_token_desired);
    uniswap.add_liquidity_cspr(Sender(owner), token, amount_token_desired, amount_cspr_desired, amount_token_min, amount_cspr_min, to, deadline.into());


    let token: Key = Key::Hash(token1.contract_hash());
    let liquidity:U256 = rng.gen_range(0..500).into();
    let amount_token_min: U256 = rng.gen_range(0..200).into();
    let amount_cspr_min: U256 = rng.gen_range(0..200).into();
    let to = Key::Hash(token2.contract_hash());

    uniswap.approve(&token1, Sender(owner), uniswap.uniswap_contract_package_hash(), amount_token_min);
    uniswap.remove_liquidity_cspr(Sender(owner), token, liquidity, amount_token_min, amount_cspr_min, to, deadline.into())
}


//#[test]
pub fn remove_liquidity_with_permit()
{
    let (env, uniswap, owner, token1, token2, token3) = deploy_uniswap_router();
    let mut rng = rand::thread_rng();

    let token_a = Key::Hash(token1.contract_hash());
    let token_b = Key::Hash(token2.contract_hash());
    let liquidity: U256 = rng.gen_range(0..500).into();
    let amount_a_min: U256 = rng.gen_range(0..200).into();
    let amount_b_min: U256 = rng.gen_range(0..200).into();
    let to = Key::Hash(token3.contract_hash());
    let approve_max = false;
    let v: u8 = rng.gen_range(0..200);
    let r: u32 = rng.gen_range(0..200);
    let s: u32 = rng.gen_range(0..200);
    let deadline: u128 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_millis() + (1000 * (30 * 60)),      // current epoch time in milisecond + 30 minutes
        Err(_) => 0
    };

    uniswap.remove_liquidity_with_permit(Sender(owner), token_a, token_b, liquidity, amount_a_min, amount_b_min, to, deadline.into(), approve_max, v, r, s);
}

/*
#[test]
fn remove_liquidity_cspr_with_permit()
{
    let (env, uniswap, owner) = deploy_uniswap_router();
    let (token1, token2, token3) = deploy_dummy_tokens(&env);
    let mut rng = rand::thread_rng();

    let token = Key::Hash(token1.contract_hash());
    let liquidity: U256 = rng.gen_range(0..500).into();
    let amount_token_min: U256 = rng.gen_range(0..500).into();
    let amount_cspr_min: U256 = rng.gen_range(0..500).into();
    let to = Key::Hash(token2.contract_hash());
    let approve_max = false;
    let v: u8 = rng.gen_range(0..200);
    let r: u32 = rng.gen_range(0..200);
    let s: u32 = rng.gen_range(0..200);
    let deadline: u128 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_millis() + (1000 * (30 * 60)),      // current epoch time in milisecond + 30 minutes
        Err(_) => 0
    };
}
*/

/*
#[test]
fn test_uniswap_swap_exact_tokens_for_tokens()
{
    let (env, token, owner) = deploy_uniswap_router();
    let user = env.next_user();
    let ret = token.swap_exact_tokens_for_tokens(200.into(), 100.into(), [].into(), user);
    println!("{:?}", ret);
}

#[test]
fn test_uniswap_swap_tokens_for_exact_tokens()
{
    let (env, token, owner) = deploy_uniswap_router();
    let user = env.next_user();
    let ret = token.swap_tokens_for_exact_tokens(200.into(), 100.into(), [].into(), user);
    println!("{:?}", ret);
}

#[test]
fn test_uniswap_swap_exact_cspr_for_tokens()
{
    let (env, token, owner) = deploy_uniswap_router();
    let user = env.next_user();
    let ret = token.swap_exact_cspr_for_tokens(200.into(), 100.into(), [].into(), user);
    println!("{:?}", ret);
}

#[test]
fn test_uniswap_swap_tokens_for_exact_cspr()
{
    let (env, token, owner) = deploy_uniswap_router();
    let user = env.next_user();
    let ret = token.swap_tokens_for_exact_cspr(200.into(), 100.into(), [].into(), user);
    println!("{:?}", ret);
}

#[test]
fn test_uniswap_swap_exact_tokens_for_cspr()
{
    let (env, token, owner) = deploy_uniswap_router();
    let user = env.next_user();
    let ret = token.swap_exact_tokens_for_cspr(200.into(), 100.into(), [].into(), user);
    println!("{:?}", ret);
}

#[test]
fn test_uniswap_swap_cspr_for_exact_tokens()
{
    let (env, token, owner) = deploy_uniswap_router();
    let user = env.next_user();
    let ret = token.swap_cspr_for_exact_tokens(200.into(), 100.into(), [].into(), user);
    println!("{:?}", ret);
}
*/