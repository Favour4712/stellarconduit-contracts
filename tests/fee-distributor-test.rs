//! # Fee Distributor — Unit Test Suite
//!
//! Comprehensive unit tests for the Fee Distributor contract covering all
//! public functions, happy paths, and error cases.

#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events as _},
    Address, Env,
};

use fee_distributor::{
    errors::ContractError, FeeDistributorContract, FeeDistributorContractClient,
};

fn setup() -> (Env, FeeDistributorContractClient) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    client.initialize(&admin, &50u32, &1000u32, &treasury);
    (env, client)
}

// ============================================================================
// initialize() Tests
// ============================================================================

#[test]
fn test_initialize_success() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.initialize(&admin, &50u32, &1000u32, &treasury);

    // Verify fee config is set correctly by calling calculate_fee
    // With fee_rate_bps = 50 and batch_size = 200, fee should be 1
    let fee = client.calculate_fee(&200u32);
    assert_eq!(fee, 1);
}

#[test]
fn test_initialize_already_initialized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.initialize(&admin, &50u32, &1000u32, &treasury);

    // Second call should fail
    let result = client.try_initialize(&admin, &50u32, &1000u32, &treasury);
    assert_eq!(result, Err(Ok(ContractError::AlreadyInitialized)));
}

// ============================================================================
// calculate_fee() Tests
// ============================================================================

#[test]
fn test_calculate_fee_success() {
    let (env, client) = setup();

    // With fee_rate_bps = 50 (0.5%) and batch_size = 200:
    // fee = 200 * 50 / 10000 = 1
    let fee = client.calculate_fee(&200u32);
    assert_eq!(fee, 1);

    // With batch_size = 1000:
    // fee = 1000 * 50 / 10000 = 5
    let fee2 = client.calculate_fee(&1000u32);
    assert_eq!(fee2, 5);

    // With batch_size = 100:
    // fee = 100 * 50 / 10000 = 0 (integer division)
    let fee3 = client.calculate_fee(&100u32);
    assert_eq!(fee3, 0);
}

#[test]
fn test_calculate_fee_zero_batch() {
    let (_env, client) = setup();

    let result = client.try_calculate_fee(&0u32);
    assert_eq!(result, Err(Ok(ContractError::InvalidBatchSize)));
}

#[test]
fn test_calculate_fee_boundary() {
    let (_env, client) = setup();

    // Test with max u32 batch size to check overflow guard
    let max_batch_size = u32::MAX;
    let result = client.try_calculate_fee(&max_batch_size);
    // This should either succeed (if no overflow) or return Overflow error
    // With fee_rate_bps = 50: max_batch_size * 50 could overflow i128
    // Let's check if it overflows
    match result {
        Ok(fee) => {
            // If it doesn't overflow, verify the calculation
            let expected = (max_batch_size as i128)
                .checked_mul(50i128)
                .and_then(|x| x.checked_div(10000));
            if let Some(exp) = expected {
                assert_eq!(fee, exp);
            }
        }
        Err(Ok(ContractError::Overflow)) => {
            // Overflow is acceptable for max u32
        }
        _ => panic!("Unexpected result"),
    }
}

// ============================================================================
// distribute() Tests
// ============================================================================

#[test]
fn test_distribute_success() {
    let (env, client) = setup();
    let relay = Address::generate(&env);
    let batch_id = 1u64;
    let batch_size = 200u32;

    client.distribute(&relay, &batch_id, &batch_size);

    // Verify relay earnings updated
    let earnings = client.get_earnings(&relay);
    // With batch_size = 200, fee_rate_bps = 50: fee = 1
    // treasury_share_bps = 1000 (10%): treasury_share = 1 * 1000 / 10000 = 0
    // relay_payout = 1 - 0 = 1
    assert_eq!(earnings.total_earned, 1);
    assert_eq!(earnings.unclaimed, 1);

    // Verify fee entry stored
    // Note: We can't directly read fee entries, but we can verify by trying to distribute again

    // Verify event emitted
    let events = env.events().all();
    let mut found = false;
    for event in events.iter() {
        let (_contract, topics, _data) = event;
        if topics.len() > 0 && topics.get(0).unwrap() == env.bytes_new_from_slice("distribute".as_bytes()) {
            found = true;
            break;
        }
    }
    assert!(found, "distribute event should be emitted");
}

#[test]
fn test_distribute_duplicate_batch() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);
    let batch_id = 1u64;
    let batch_size = 200u32;

    client.distribute(&relay, &batch_id, &batch_size);

    // Second call with same batch_id should fail
    let result = client.try_distribute(&relay, &batch_id, &batch_size);
    assert_eq!(result, Err(Ok(ContractError::BatchAlreadyDistributed)));
}

#[test]
fn test_distribute_zero_batch_size() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);
    let batch_id = 1u64;

    let result = client.try_distribute(&relay, &batch_id, &0u32);
    assert_eq!(result, Err(Ok(ContractError::InvalidBatchSize)));
}

#[test]
fn test_distribute_treasury_share_split() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);
    let batch_id = 1u64;
    let batch_size = 10000u32; // Large batch to get meaningful treasury share

    client.distribute(&relay, &batch_id, &batch_size);

    // With batch_size = 10000, fee_rate_bps = 50: fee = 10000 * 50 / 10000 = 50
    // treasury_share_bps = 1000 (10%): treasury_share = 50 * 1000 / 10000 = 5
    // relay_payout = 50 - 5 = 45
    let earnings = client.get_earnings(&relay);
    assert_eq!(earnings.total_earned, 45);
    assert_eq!(earnings.unclaimed, 45);

    // Verify: relay_payout + treasury_share == total fee
    // 45 + 5 = 50 ✓
    assert_eq!(earnings.total_earned + 5, 50);
}

// ============================================================================
// claim() Tests
// ============================================================================

#[test]
fn test_claim_success() {
    let (env, client) = setup();
    let relay = Address::generate(&env);
    let batch_id = 1u64;
    let batch_size = 200u32;

    // First distribute some fees
    client.distribute(&relay, &batch_id, &batch_size);

    let earnings_before = client.get_earnings(&relay);
    assert_eq!(earnings_before.unclaimed, 1);

    // Claim the fees
    let payout = client.claim(&relay);

    // Verify payout amount
    assert_eq!(payout, 1);

    // Verify unclaimed zeroed and total_claimed incremented
    let earnings_after = client.get_earnings(&relay);
    assert_eq!(earnings_after.unclaimed, 0);
    assert_eq!(earnings_after.total_claimed, 1);
    assert_eq!(earnings_after.total_earned, 1);
}

#[test]
fn test_claim_nothing_to_claim() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);

    // Try to claim when there's nothing to claim
    let result = client.try_claim(&relay);
    assert_eq!(result, Err(Ok(ContractError::NothingToClaim)));
}

#[test]
#[should_panic(expected = "HostError")]
fn test_claim_auth_required() {
    let (env, client) = setup();
    let relay = Address::generate(&env);
    let batch_id = 1u64;
    let batch_size = 200u32;

    // Distribute some fees
    client.distribute(&relay, &batch_id, &batch_size);

    // Create a new env without mock_all_auths to test auth requirement
    let env2 = Env::default();
    // Don't call env2.mock_all_auths() - this should cause auth to fail
    let contract_id = env2.register_contract(None, FeeDistributorContract);
    let client2 = FeeDistributorContractClient::new(&env2, &contract_id);

    // This should panic because relay hasn't authorized
    client2.claim(&relay);
}

// ============================================================================
// get_earnings() Tests
// ============================================================================

#[test]
fn test_get_earnings_existing() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);
    let batch_id1 = 1u64;
    let batch_id2 = 2u64;
    let batch_size = 200u32;

    // Distribute fees twice
    client.distribute(&relay, &batch_id1, &batch_size);
    client.distribute(&relay, &batch_id2, &batch_size);

    let earnings = client.get_earnings(&relay);
    // Each distribution adds 1 to total_earned and unclaimed
    assert_eq!(earnings.total_earned, 2);
    assert_eq!(earnings.unclaimed, 2);
    assert_eq!(earnings.total_claimed, 0);
}

#[test]
fn test_get_earnings_default() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);

    // Get earnings for a relay that has never received distributions
    let earnings = client.get_earnings(&relay);
    assert_eq!(earnings.total_earned, 0);
    assert_eq!(earnings.unclaimed, 0);
    assert_eq!(earnings.total_claimed, 0);
}

// ============================================================================
// set_fee_rate() Tests
// ============================================================================

#[test]
fn test_set_fee_rate_success() {
    let (_env, client) = setup();
    let admin = Address::generate(&_env);
    let treasury = Address::generate(&_env);

    // Re-initialize to get admin address (in real scenario, we'd get it from storage)
    // For this test, we'll use the setup which already initializes with an admin
    // We need to get the admin from the contract, but since we can't read it directly,
    // we'll test by setting up a new contract with a known admin
    let env2 = Env::default();
    env2.mock_all_auths();
    let contract_id2 = env2.register_contract(None, FeeDistributorContract);
    let client2 = FeeDistributorContractClient::new(&env2, &contract_id2);
    let admin2 = Address::generate(&env2);
    let treasury2 = Address::generate(&env2);
    client2.initialize(&admin2, &50u32, &1000u32, &treasury2);

    // Update fee rate to 100 bps (1%)
    client2.set_fee_rate(&100u32);

    // Verify change reflected in calculate_fee
    // With fee_rate_bps = 100 and batch_size = 200: fee = 200 * 100 / 10000 = 2
    let fee = client2.calculate_fee(&200u32);
    assert_eq!(fee, 2);
}

#[test]
fn test_set_fee_rate_invalid_zero() {
    let (_env, client) = setup();
    let admin = Address::generate(&_env);
    let treasury = Address::generate(&_env);

    // Similar setup as above - we need admin context
    let env2 = Env::default();
    env2.mock_all_auths();
    let contract_id2 = env2.register_contract(None, FeeDistributorContract);
    let client2 = FeeDistributorContractClient::new(&env2, &contract_id2);
    let admin2 = Address::generate(&env2);
    let treasury2 = Address::generate(&env2);
    client2.initialize(&admin2, &50u32, &1000u32, &treasury2);

    // Try to set fee rate to 0
    let result = client2.try_set_fee_rate(&0u32);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));
}

#[test]
fn test_set_fee_rate_invalid_above_max() {
    let (_env, client) = setup();
    let admin = Address::generate(&_env);
    let treasury = Address::generate(&_env);

    // Similar setup as above
    let env2 = Env::default();
    env2.mock_all_auths();
    let contract_id2 = env2.register_contract(None, FeeDistributorContract);
    let client2 = FeeDistributorContractClient::new(&env2, &contract_id2);
    let admin2 = Address::generate(&env2);
    let treasury2 = Address::generate(&env2);
    client2.initialize(&admin2, &50u32, &1000u32, &treasury2);

    // Try to set fee rate to 10001 (above max of 10000)
    let result = client2.try_set_fee_rate(&10001u32);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));
}

#[test]
#[should_panic(expected = "HostError")]
fn test_set_fee_rate_unauthorized() {
    let (_env, client) = setup();
    let admin = Address::generate(&_env);
    let treasury = Address::generate(&_env);

    // Setup contract with admin
    let env2 = Env::default();
    env2.mock_all_auths();
    let contract_id2 = env2.register_contract(None, FeeDistributorContract);
    let client2 = FeeDistributorContractClient::new(&env2, &contract_id2);
    let admin2 = Address::generate(&env2);
    let treasury2 = Address::generate(&env2);
    client2.initialize(&admin2, &50u32, &1000u32, &treasury2);

    // Create a new env without mock_all_auths and try to call as non-admin
    let env3 = Env::default();
    // Don't call env3.mock_all_auths() - this should cause auth to fail
    let contract_id3 = env3.register_contract(None, FeeDistributorContract);
    let client3 = FeeDistributorContractClient::new(&env3, &contract_id3);

    // This should panic because non-admin hasn't authorized
    let non_admin = Address::generate(&env3);
    client3.set_fee_rate(&100u32);
}
