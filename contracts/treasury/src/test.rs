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
    assert_eq!(balance_of(&env, &client.address), 0);

    client.deposit(&from, &1000);
    assert_eq!(balance_of(&env, &client.address), 1000);
}

#[test]
fn test_deposit_logs_entry_and_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let from = Address::generate(&env);

    client.deposit(&from, &500);

    // Val doesn't implement PartialEq; verify count and emitting contract only.
    let events = env.events().all();
    assert_eq!(events.len(), 1);
    let (emitting_contract, _topics, _data) = events.first().unwrap();
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
    env.as_contract(&client.address, || storage::set_admin(&env, &admin));
    client.deposit(&admin, &5000);

    client.withdraw(&to, &1000, &String::from_str(&env, "test"));
    assert_eq!(balance_of(&env, &client.address), 4000);
}

#[test]
#[should_panic]
fn test_withdraw_unauthorized() {
    let env = Env::default();
    // Deliberately no mock_all_auths — admin.require_auth() inside withdraw will panic.
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);
    let to = Address::generate(&env);
    env.as_contract(&client.address, || storage::set_admin(&env, &admin));
    client.withdraw(&to, &1000, &String::from_str(&env, "test"));
}

#[test]
fn test_withdraw_insufficient_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let client = create_treasury_contract(&env);
    let admin = Address::generate(&env);
    let to = Address::generate(&env);
    env.as_contract(&client.address, || storage::set_admin(&env, &admin));
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
    env.as_contract(&client.address, || {
        storage::set_admin(&env, &admin);
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
    env.as_contract(&client.address, || storage::set_admin(&env, &admin));
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
    env.as_contract(&client.address, || {
        storage::set_admin(&env, &admin);
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
    env.as_contract(&client.address, || {
        storage::set_admin(&env, &admin);
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
    env.as_contract(&client.address, || {
        storage::set_admin(&env, &admin);
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
