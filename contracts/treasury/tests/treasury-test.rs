#![cfg(test)]

use soroban_sdk::{testutils::Address as _, token, Address, Env, String};
use treasury::{types::SpendingProgram, TreasuryContract, TreasuryContractClient};

/// Sets up the test environment with a mocked SAC token contract.
fn setup<'a>() -> (
    Env,
    TreasuryContractClient<'a>,
    Address,
    Address,
    token::StellarAssetClient<'a>,
) {
    let env = Env::default();
    env.mock_all_auths();

    // Register the treasury contract
    let contract_id = env.register(TreasuryContract, ());
    let client = TreasuryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    // Deploy a mock SAC token contract required for deposit/withdraw tests
    let token_admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());

    // Initialize the treasury
    client.initialize(&admin, &token_address.address());

    (env, client, admin, token_address.address(), token_client)
}

/// Helper to create and persist a spending program directly into storage.
/// This sidesteps the lack of a public `create_program` function by invoking
/// the private storage helpers within the contract environment context.
fn create_spending_program(
    env: &Env,
    client: &TreasuryContractClient,
    program_id: u64,
    budget: i128,
) {
    let program = SpendingProgram {
        program_id,
        budget,
        spent: 0,
        active: true,
        name: String::from_str(env, "Test Program"),
    };

    env.as_contract(&client.address, || {
        treasury::storage::set_spending_program(env, program_id, program);
    });
}

// ── initialize() tests ────────────────────────────────────────────────────────

#[test]
fn test_initialize_success() {
    let env = Env::default();
    let contract_id = env.register(TreasuryContract, ());
    let client = TreasuryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token);

    // Verify balance is initialized to 0
    assert_eq!(client.get_balance(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_initialize_already_initialized() {
    let env = Env::default();
    let contract_id = env.register(TreasuryContract, ());
    let client = TreasuryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // First call succeeds
    client.initialize(&admin, &token);

    // Second call should panic with AlreadyInitialized
    client.initialize(&admin, &token);
}

// ── deposit() tests ───────────────────────────────────────────────────────────

#[test]
fn test_deposit_success() {
    let (env, client, _admin, token_id, token_client) = setup();
    let user = Address::generate(&env);

    // Mint tokens to the user
    token_client.mint(&user, &10_000);

    // Deposit 5,000
    client.deposit(&user, &5_000);

    assert_eq!(client.get_balance(), 5_000);

    let token = token::Client::new(&env, &token_id);
    assert_eq!(token.balance(&user), 5_000);
    assert_eq!(token.balance(&client.address), 5_000);
}

#[test]
fn test_deposit_multiple() {
    let (env, client, _admin, _token_id, token_client) = setup();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    token_client.mint(&user1, &500);
    token_client.mint(&user2, &1_500);

    client.deposit(&user1, &300);
    client.deposit(&user2, &700);

    assert_eq!(client.get_balance(), 1_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_deposit_zero_amount() {
    let (env, client, _admin, _token_id, _) = setup();
    let user = Address::generate(&env);
    client.deposit(&user, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_deposit_negative_amount() {
    let (env, client, _admin, _token_id, _) = setup();
    let user = Address::generate(&env);
    client.deposit(&user, &-100);
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn test_deposit_auth_required() {
    let env = Env::default();
    // Do NOT call env.mock_all_auths() here so require_auth() will fail
    let contract_id = env.register(TreasuryContract, ());
    let client = TreasuryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());

    client.initialize(&admin, &token_address.address());

    let user = Address::generate(&env);
    // This will panic because require_auth is called on `user` but we didn't mock/provide it
    client.deposit(&user, &1_000);
}

// ── withdraw() tests ──────────────────────────────────────────────────────────

#[test]
fn test_withdraw_success() {
    let (env, client, _admin, token_id, token_client) = setup();
    let user = Address::generate(&env);

    // First deposit 10,000 into treasury
    token_client.mint(&user, &10_000);
    client.deposit(&user, &10_000);

    let recipient = Address::generate(&env);
    client.withdraw(&recipient, &3_000, &String::from_str(&env, "grant"));

    assert_eq!(client.get_balance(), 7_000);

    let token = token::Client::new(&env, &token_id);
    assert_eq!(token.balance(&recipient), 3_000);
    assert_eq!(token.balance(&client.address), 7_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_withdraw_insufficient_balance() {
    let (env, client, _admin, _token_id, token_client) = setup();
    let user = Address::generate(&env);

    // Deposit only 2,000
    token_client.mint(&user, &2_000);
    client.deposit(&user, &2_000);

    let recipient = Address::generate(&env);
    // Try to withdraw 5,000
    client.withdraw(&recipient, &5_000, &String::from_str(&env, "grant"));
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_withdraw_zero_amount() {
    let (env, client, _admin, _token_id, _) = setup();
    let recipient = Address::generate(&env);
    client.withdraw(&recipient, &0, &String::from_str(&env, "grant"));
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn test_withdraw_unauthorized() {
    let env = Env::default();
    let contract_id = env.register(TreasuryContract, ());
    let client = TreasuryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());

    client.initialize(&admin, &token_address.address());

    let recipient = Address::generate(&env);
    // Fails because admin.require_auth() is called, but auth is not mocked
    client.withdraw(&recipient, &100, &String::from_str(&env, "grant"));
}

// ── allocate() tests ──────────────────────────────────────────────────────────

#[test]
fn test_allocate_success() {
    let (env, client, _admin, _token_id, token_client) = setup();
    let user = Address::generate(&env);

    // Deposit 10,000
    token_client.mint(&user, &10_000);
    client.deposit(&user, &10_000);

    // Create spending program 1 with budget 5000
    create_spending_program(&env, &client, 1, 5_000);

    // Allocate 2000
    client.allocate(&1, &2_000);

    assert_eq!(client.get_balance(), 8_000);
    // In actual contract, allocate() increments spent
    let program = env.as_contract(&client.address, || {
        treasury::storage::get_spending_program(&env, 1).unwrap()
    });
    assert_eq!(program.spent, 2_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_allocate_program_not_found() {
    let (env, client, _admin, _token_id, token_client) = setup();
    let user = Address::generate(&env);

    token_client.mint(&user, &10_000);
    client.deposit(&user, &10_000);

    // Program 1 does not exist
    client.allocate(&1, &2_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_allocate_program_inactive() {
    let (env, client, _admin, _token_id, token_client) = setup();
    let user = Address::generate(&env);

    token_client.mint(&user, &10_000);
    client.deposit(&user, &10_000);

    // Create inactive program
    let program = treasury::types::SpendingProgram {
        program_id: 1,
        budget: 5_000,
        spent: 0,
        active: false,
        name: String::from_str(&env, "Inactive"),
    };
    env.as_contract(&client.address, || {
        treasury::storage::set_spending_program(&env, 1, program);
    });

    client.allocate(&1, &2_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_allocate_over_budget() {
    let (env, client, _admin, _token_id, token_client) = setup();
    let user = Address::generate(&env);

    token_client.mint(&user, &10_000);
    client.deposit(&user, &10_000);

    // Create program with budget 5_000
    create_spending_program(&env, &client, 1, 5_000);

    // Try to allocate 6_000
    client.allocate(&1, &6_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_allocate_insufficient_treasury_balance() {
    let (env, client, _admin, _token_id, token_client) = setup();
    let user = Address::generate(&env);

    // Deposit only 2,000
    token_client.mint(&user, &2_000);
    client.deposit(&user, &2_000);

    // Create program with budget 5_000
    create_spending_program(&env, &client, 1, 5_000);

    // Try to allocate 3_000 (below budget, but exceeds treasury balance)
    client.allocate(&1, &3_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn test_allocate_unauthorized() {
    let env = Env::default();
    let contract_id = env.register(TreasuryContract, ());
    let client = TreasuryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract_v2(token_admin.clone());

    client.initialize(&admin, &token_address.address());

    // Fails because admin.require_auth() is called, but auth is not mocked
    client.allocate(&1, &100);
}

// ── get_balance() tests ───────────────────────────────────────────────────────

#[test]
fn test_get_balance_initial() {
    let (_env, client, _admin, _token_id, _) = setup();
    assert_eq!(client.get_balance(), 0);
}

#[test]
fn test_get_balance_after_deposit() {
    let (env, client, _admin, _token_id, token_client) = setup();
    let user = Address::generate(&env);

    token_client.mint(&user, &1_500);
    client.deposit(&user, &1_500);

    assert_eq!(client.get_balance(), 1_500);
}

#[test]
fn test_get_balance_after_withdraw() {
    let (env, client, _admin, _token_id, token_client) = setup();
    let user = Address::generate(&env);

    token_client.mint(&user, &5_000);
    client.deposit(&user, &5_000);

    let recipient = Address::generate(&env);
    client.withdraw(&recipient, &2_000, &String::from_str(&env, "grant"));

    assert_eq!(client.get_balance(), 3_000);
}

// ── get_history() tests ───────────────────────────────────────────────────────

#[test]
fn test_get_history_deposit_entry() {
    let (env, client, _admin, _token_id, token_client) = setup();
    let user = Address::generate(&env);

    token_client.mint(&user, &5_000);
    client.deposit(&user, &5_000);

    // We can query ID 1 directly (first entry)
    let entry = client.get_history(&1);

    assert_eq!(entry.amount, 5_000);
    assert_eq!(entry.actor, user);
    assert!(entry.recipient.is_none());
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_get_history_not_found() {
    let (_env, client, _admin, _token_id, _) = setup();

    // No history yet, requesting ID 999 should error
    client.get_history(&999);
}
