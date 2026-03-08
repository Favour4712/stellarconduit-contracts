//! # Treasury — Integration Test Suite
//!
//! Unit and integration tests for the Protocol Treasury contract.

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events as _},
    Address, Env, String,
};

// The contract-under-test.
use crate::{storage, types::SpendingProgram, TreasuryContract, TreasuryContractClient};

use soroban_sdk::token::StellarAssetClient;

fn create_token_contract<'a>(env: &Env, admin: &Address) -> (StellarAssetClient<'a>, Address) {
    let contract_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (StellarAssetClient::new(env, &contract_id), contract_id)
}

fn create_treasury_contract<'a>(env: &Env) -> TreasuryContractClient<'a> {
    let contract_id = env.register(TreasuryContract, ());
    TreasuryContractClient::new(env, &contract_id)
}

/// Helper: read the treasury balance from inside the contract's storage context.
fn balance_of(env: &Env, contract: &Address) -> i128 {
    env.as_contract(contract, || storage::get_balance(env))
}

#[test]
fn test_deposit_increases_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let from = Address::generate(&env);

    let (token_client, token_address) = create_token_contract(&env, &from);
    token_client.mint(&from, &10000);

    env.as_contract(&client.address, || {
        storage::set_token_address(&env, &token_address);
    });
    assert_eq!(balance_of(&env, &client.address), 0);

    client.deposit(&from, &1000);
    assert_eq!(balance_of(&env, &client.address), 1000);

    let token_balance =
        soroban_sdk::token::Client::new(&env, &token_address).balance(&client.address);
    assert_eq!(token_balance, 1000);
}

#[test]
fn test_deposit_logs_entry_and_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let from = Address::generate(&env);
    let (token_client, token_address) = create_token_contract(&env, &from);
    token_client.mint(&from, &10000);

    env.as_contract(&client.address, || {
        storage::set_token_address(&env, &token_address);
    });

    client.deposit(&from, &500);

    // Note: the token transfer also emits an event. The treasury logic emits the SECOND event.
    let events = env.events().all();
    assert_eq!(events.len(), 2);
    let (emitting_contract, _topics, _data) = events.get(1).unwrap();
    assert_eq!(emitting_contract, client.address);
}

#[test]
fn test_deposit_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let from = Address::generate(&env);
    let res = client.try_deposit(&from, &0);
    assert_eq!(res, Err(Ok(crate::errors::ContractError::InvalidAmount)));
}

#[test]
fn test_withdraw_by_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);
    let to = Address::generate(&env);

    let (token_client, token_address) = create_token_contract(&env, &admin);
    token_client.mint(&admin, &10000);

    env.as_contract(&client.address, || {
        let mut members = soroban_sdk::Vec::new(&env);
        members.push_back(admin.clone());
        let council = crate::types::AdminCouncil {
            members,
            threshold: 1,
        };
        storage::set_admin_council(&env, &council);
        storage::set_token_address(&env, &token_address);
    });
    client.deposit(&admin, &5000);

    client.withdraw(&to, &1000, &String::from_str(&env, "test"));
    assert_eq!(balance_of(&env, &client.address), 4000);

    let token_balance = soroban_sdk::token::Client::new(&env, &token_address).balance(&to);
    assert_eq!(token_balance, 1000);
}

#[test]
#[should_panic]
fn test_withdraw_unauthorized() {
    let env = Env::default();
    // Deliberately no mock_all_auths — admin.require_auth() inside withdraw will panic.
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);
    let to = Address::generate(&env);
    let (_, token_address) = create_token_contract(&env, &admin);
    env.as_contract(&client.address, || {
        let mut members = soroban_sdk::Vec::new(&env);
        members.push_back(admin.clone());
        let council = crate::types::AdminCouncil {
            members,
            threshold: 1,
        };
        storage::set_admin_council(&env, &council);
        storage::set_token_address(&env, &token_address);
    });
    client.withdraw(&to, &1000, &String::from_str(&env, "test"));
}

#[test]
fn test_withdraw_insufficient_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);
    let to = Address::generate(&env);

    let (token_client, token_address) = create_token_contract(&env, &admin);
    token_client.mint(&admin, &10000);

    env.as_contract(&client.address, || {
        let mut members = soroban_sdk::Vec::new(&env);
        members.push_back(admin.clone());
        let council = crate::types::AdminCouncil {
            members,
            threshold: 1,
        };
        storage::set_admin_council(&env, &council);
        storage::set_token_address(&env, &token_address);
    });
    client.deposit(&admin, &100);

    let res = client.try_withdraw(&to, &200, &String::from_str(&env, "test"));
    assert_eq!(
        res,
        Err(Ok(crate::errors::ContractError::InsufficientBalance))
    );
}

#[test]
fn test_allocate_by_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);

    let (token_client, token_address) = create_token_contract(&env, &admin);
    token_client.mint(&admin, &20000);

    env.as_contract(&client.address, || {
        let mut members = soroban_sdk::Vec::new(&env);
        members.push_back(admin.clone());
        let council = crate::types::AdminCouncil {
            members,
            threshold: 1,
        };
        storage::set_admin_council(&env, &council);
        storage::set_token_address(&env, &token_address);
        let program = SpendingProgram {
            program_id: 1,
            name: String::from_str(&env, "Test Program"),
            budget: 5000,
            spent: 0,
            active: true,
        };
        storage::set_spending_program(&env, 1, program);
    });
    client.deposit(&admin, &10000);

    client.allocate(&1, &2000);

    let (spent, bal) = env.as_contract(&client.address, || {
        let prog = storage::get_spending_program(&env, 1).unwrap();
        (prog.spent, storage::get_balance(&env))
    });
    assert_eq!(spent, 2000);
    assert_eq!(bal, 8000);
}

#[test]
fn test_allocate_program_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);

    let (token_client, token_address) = create_token_contract(&env, &admin);
    token_client.mint(&admin, &20000);

    env.as_contract(&client.address, || {
        let mut members = soroban_sdk::Vec::new(&env);
        members.push_back(admin.clone());
        let council = crate::types::AdminCouncil {
            members,
            threshold: 1,
        };
        storage::set_admin_council(&env, &council);
        storage::set_token_address(&env, &token_address);
    });
    client.deposit(&admin, &10000);

    let res = client.try_allocate(&99, &1000);
    assert_eq!(res, Err(Ok(crate::errors::ContractError::ProgramNotFound)));
}

#[test]
fn test_allocate_program_inactive() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);

    let (token_client, token_address) = create_token_contract(&env, &admin);
    token_client.mint(&admin, &20000);

    env.as_contract(&client.address, || {
        let mut members = soroban_sdk::Vec::new(&env);
        members.push_back(admin.clone());
        let council = crate::types::AdminCouncil {
            members,
            threshold: 1,
        };
        storage::set_admin_council(&env, &council);
        storage::set_token_address(&env, &token_address);
        let program = SpendingProgram {
            program_id: 1,
            name: String::from_str(&env, "Test Program"),
            budget: 5000,
            spent: 0,
            active: false, // inactive
        };
        storage::set_spending_program(&env, 1, program);
    });
    client.deposit(&admin, &10000);

    let res = client.try_allocate(&1, &1000);
    assert_eq!(res, Err(Ok(crate::errors::ContractError::ProgramInactive)));
}

#[test]
fn test_allocate_over_budget() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);

    let (token_client, token_address) = create_token_contract(&env, &admin);
    token_client.mint(&admin, &20000);

    env.as_contract(&client.address, || {
        let mut members = soroban_sdk::Vec::new(&env);
        members.push_back(admin.clone());
        let council = crate::types::AdminCouncil {
            members,
            threshold: 1,
        };
        storage::set_admin_council(&env, &council);
        storage::set_token_address(&env, &token_address);
        let program = SpendingProgram {
            program_id: 1,
            name: String::from_str(&env, "Test Program"),
            budget: 5000,
            spent: 4000,
            active: true,
        };
        storage::set_spending_program(&env, 1, program);
    });
    client.deposit(&admin, &10000);

    let res = client.try_allocate(&1, &1500);
    assert_eq!(
        res,
        Err(Ok(crate::errors::ContractError::ProgramOverBudget))
    );
}

#[test]
fn test_allocate_insufficient_treasury_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);

    let (token_client, token_address) = create_token_contract(&env, &admin);
    token_client.mint(&admin, &20000);

    env.as_contract(&client.address, || {
        let mut members = soroban_sdk::Vec::new(&env);
        members.push_back(admin.clone());
        let council = crate::types::AdminCouncil {
            members,
            threshold: 1,
        };
        storage::set_admin_council(&env, &council);
        storage::set_token_address(&env, &token_address);
        let program = SpendingProgram {
            program_id: 1,
            name: String::from_str(&env, "Test Program"),
            budget: 8000,
            spent: 0,
            active: true,
        };
        storage::set_spending_program(&env, 1, program);
    });
    client.deposit(&admin, &3000); // Not enough for the allocation

    let res = client.try_allocate(&1, &5000);
    assert_eq!(
        res,
        Err(Ok(crate::errors::ContractError::InsufficientBalance))
    );
}

#[test]
fn test_get_treasury_stats_default() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);

    // Before any operations, stats should be all zeros
    let stats = client.get_treasury_stats();
    assert_eq!(stats.current_balance, 0);
    assert_eq!(stats.lifetime_deposited, 0);
    assert_eq!(stats.lifetime_withdrawn, 0);
    assert_eq!(stats.lifetime_allocated, 0);
}

#[test]
fn test_get_treasury_stats_after_deposit() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let from = Address::generate(&env);

    let (token_client, token_address) = create_token_contract(&env, &from);
    token_client.mint(&from, &10000);

    env.as_contract(&client.address, || {
        storage::set_token_address(&env, &token_address);
    });

    client.deposit(&from, &1000);
    let stats = client.get_treasury_stats();
    assert_eq!(stats.current_balance, 1000);
    assert_eq!(stats.lifetime_deposited, 1000);
    assert_eq!(stats.lifetime_withdrawn, 0);
    assert_eq!(stats.lifetime_allocated, 0);

    // Second deposit
    client.deposit(&from, &2000);
    let stats = client.get_treasury_stats();
    assert_eq!(stats.current_balance, 3000);
    assert_eq!(stats.lifetime_deposited, 3000);
}

#[test]
fn test_get_treasury_stats_after_withdraw() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);
    let to = Address::generate(&env);

    let (token_client, token_address) = create_token_contract(&env, &admin);
    token_client.mint(&admin, &10000);

    env.as_contract(&client.address, || {
        let mut members = soroban_sdk::Vec::new(&env);
        members.push_back(admin.clone());
        let council = crate::types::AdminCouncil {
            members,
            threshold: 1,
        };
        storage::set_admin_council(&env, &council);
        storage::set_token_address(&env, &token_address);
    });

    client.deposit(&admin, &5000);
    client.withdraw(&to, &1500, &String::from_str(&env, "test withdraw"));

    let stats = client.get_treasury_stats();
    assert_eq!(stats.current_balance, 3500);
    assert_eq!(stats.lifetime_deposited, 5000);
    assert_eq!(stats.lifetime_withdrawn, 1500);
    assert_eq!(stats.lifetime_allocated, 0);
}

#[test]
fn test_get_treasury_stats_after_allocate() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);

    let (token_client, token_address) = create_token_contract(&env, &admin);
    token_client.mint(&admin, &20000);

    env.as_contract(&client.address, || {
        let mut members = soroban_sdk::Vec::new(&env);
        members.push_back(admin.clone());
        let council = crate::types::AdminCouncil {
            members,
            threshold: 1,
        };
        storage::set_admin_council(&env, &council);
        storage::set_token_address(&env, &token_address);
        let program = SpendingProgram {
            program_id: 1,
            name: String::from_str(&env, "Test Program"),
            budget: 5000,
            spent: 0,
            active: true,
        };
        storage::set_spending_program(&env, 1, program);
    });

    client.deposit(&admin, &10000);
    client.allocate(&1, &2000);

    let stats = client.get_treasury_stats();
    assert_eq!(stats.current_balance, 8000);
    assert_eq!(stats.lifetime_deposited, 10000);
    assert_eq!(stats.lifetime_withdrawn, 0);
    assert_eq!(stats.lifetime_allocated, 2000);
}

#[test]
fn test_get_treasury_stats_full_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);
    let to = Address::generate(&env);

    let (token_client, token_address) = create_token_contract(&env, &admin);
    token_client.mint(&admin, &50000);

    env.as_contract(&client.address, || {
        let mut members = soroban_sdk::Vec::new(&env);
        members.push_back(admin.clone());
        let council = crate::types::AdminCouncil {
            members,
            threshold: 1,
        };
        storage::set_admin_council(&env, &council);
        storage::set_token_address(&env, &token_address);
        let program = SpendingProgram {
            program_id: 1,
            name: String::from_str(&env, "Test Program"),
            budget: 10000,
            spent: 0,
            active: true,
        };
        storage::set_spending_program(&env, 1, program);
    });

    // Multiple deposits
    client.deposit(&admin, &10000);
    client.deposit(&admin, &5000);

    // Withdrawal
    client.withdraw(&to, &3000, &String::from_str(&env, "grant"));

    // Allocation
    client.allocate(&1, &4000);

    // Final stats check
    let stats = client.get_treasury_stats();
    assert_eq!(stats.current_balance, 8000); // 15000 - 3000 - 4000
    assert_eq!(stats.lifetime_deposited, 15000); // 10000 + 5000
    assert_eq!(stats.lifetime_withdrawn, 3000);
    assert_eq!(stats.lifetime_allocated, 4000);
}
