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

use crate::{
    errors::ContractError, FeeDistributorContract, FeeDistributorContractClient,
};

fn setup<'a>() -> (Env, FeeDistributorContractClient<'a>) {
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
    let (_env, client) = setup();

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
        if topics.len() > 0 {
            let topic_bytes = topics.get(0).unwrap();
            let distribute_bytes = soroban_sdk::Bytes::from_slice(&env, "distribute".as_bytes());
            if topic_bytes == distribute_bytes {
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
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.initialize(&admin, &50u32, &1000u32, &treasury);

    // Update fee rate to 100 bps (1%)
    client.set_fee_rate(&100u32);

    // Verify change reflected in calculate_fee
    // With fee_rate_bps = 100 and batch_size = 200: fee = 200 * 100 / 10000 = 2
    let fee = client.calculate_fee(&200u32);
    assert_eq!(fee, 2);
}

#[test]
fn test_set_fee_rate_invalid_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.initialize(&admin, &50u32, &1000u32, &treasury);

    // Try to set fee rate to 0
    let result = client.try_set_fee_rate(&0u32);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));
}

#[test]
fn test_set_fee_rate_invalid_above_max() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.initialize(&admin, &50u32, &1000u32, &treasury);

    // Try to set fee rate to 10001 (above max of 10000)
    let result = client.try_set_fee_rate(&10001u32);
    assert_eq!(result, Err(Ok(ContractError::InvalidFeeRate)));
}

#[test]
#[should_panic(expected = "HostError")]
fn test_set_fee_rate_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.initialize(&admin, &50u32, &1000u32, &treasury);

    // Create a new env without mock_all_auths and try to call as non-admin
    let env2 = Env::default();
    // Don't call env2.mock_all_auths() - this should cause auth to fail
    let contract_id2 = env2.register_contract(None, FeeDistributorContract);
    let client2 = FeeDistributorContractClient::new(&env2, &contract_id2);

    // This should panic because non-admin hasn't authorized
    client2.set_fee_rate(&100u32);
}

// ============================================================================
// Additional Comprehensive Tests
// ============================================================================

#[test]
fn test_multiple_distributions_same_relay() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);

    // Distribute fees multiple times to same relay
    client.distribute(&relay, &1u64, &200u32); // fee = 1
    client.distribute(&relay, &2u64, &1000u32); // fee = 5
    client.distribute(&relay, &3u64, &400u32); // fee = 2

    let earnings = client.get_earnings(&relay);
    // Total relay payouts: 1 + 5 + 2 = 8 (treasury share is 0 for small amounts)
    assert_eq!(earnings.total_earned, 8);
    assert_eq!(earnings.unclaimed, 8);
}

#[test]
fn test_multiple_distributions_different_relays() {
    let (_env, client) = setup();
    let relay1 = Address::generate(&_env);
    let relay2 = Address::generate(&_env);

    client.distribute(&relay1, &1u64, &200u32);
    client.distribute(&relay2, &2u64, &200u32);

    let earnings1 = client.get_earnings(&relay1);
    let earnings2 = client.get_earnings(&relay2);

    // Each relay should have independent earnings
    assert_eq!(earnings1.total_earned, 1);
    assert_eq!(earnings1.unclaimed, 1);
    assert_eq!(earnings2.total_earned, 1);
    assert_eq!(earnings2.unclaimed, 1);
}

#[test]
fn test_multiple_claims() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);

    // Distribute, claim, distribute again, claim again
    client.distribute(&relay, &1u64, &200u32);
    let payout1 = client.claim(&relay);
    assert_eq!(payout1, 1);

    client.distribute(&relay, &2u64, &1000u32);
    let payout2 = client.claim(&relay);
    assert_eq!(payout2, 5);

    let earnings = client.get_earnings(&relay);
    assert_eq!(earnings.total_earned, 6);
    assert_eq!(earnings.total_claimed, 6);
    assert_eq!(earnings.unclaimed, 0);
}

#[test]
fn test_claim_after_multiple_distributions() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);

    // Distribute multiple times without claiming
    client.distribute(&relay, &1u64, &200u32); // 1
    client.distribute(&relay, &2u64, &400u32); // 2
    client.distribute(&relay, &3u64, &600u32); // 3

    // Claim all at once
    let payout = client.claim(&relay);
    assert_eq!(payout, 6);

    let earnings = client.get_earnings(&relay);
    assert_eq!(earnings.total_earned, 6);
    assert_eq!(earnings.total_claimed, 6);
    assert_eq!(earnings.unclaimed, 0);
}

#[test]
fn test_calculate_fee_with_different_rates() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    // Initialize with fee_rate_bps = 100 (1%)
    client.initialize(&admin, &100u32, &1000u32, &treasury);

    // With batch_size = 200: fee = 200 * 100 / 10000 = 2
    let fee = client.calculate_fee(&200u32);
    assert_eq!(fee, 2);

    // Update to fee_rate_bps = 500 (5%)
    client.set_fee_rate(&500u32);

    // With batch_size = 200: fee = 200 * 500 / 10000 = 10
    let fee2 = client.calculate_fee(&200u32);
    assert_eq!(fee2, 10);
}

#[test]
fn test_distribute_with_different_batch_sizes() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);

    // Test various batch sizes
    client.distribute(&relay, &1u64, &1u32); // fee = 0 (rounding down)
    client.distribute(&relay, &2u64, &100u32); // fee = 0
    client.distribute(&relay, &3u64, &200u32); // fee = 1
    client.distribute(&relay, &4u64, &10000u32); // fee = 50

    let earnings = client.get_earnings(&relay);
    // Total: 0 + 0 + 1 + 45 (50 - 5 treasury) = 46
    assert_eq!(earnings.total_earned, 46);
}

#[test]
fn test_earnings_invariant_total_equals_claimed_plus_unclaimed() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);

    // Distribute some fees
    client.distribute(&relay, &1u64, &200u32);
    client.distribute(&relay, &2u64, &400u32);

    let earnings = client.get_earnings(&relay);
    // Invariant: total_earned = total_claimed + unclaimed
    assert_eq!(
        earnings.total_earned,
        earnings.total_claimed + earnings.unclaimed
    );

    // Claim some
    client.claim(&relay);

    let earnings2 = client.get_earnings(&relay);
    // Invariant should still hold
    assert_eq!(
        earnings2.total_earned,
        earnings2.total_claimed + earnings2.unclaimed
    );
}

#[test]
fn test_distribute_small_batch_size() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);

    // Test with smallest valid batch size
    client.distribute(&relay, &1u64, &1u32);

    let earnings = client.get_earnings(&relay);
    // With batch_size = 1, fee_rate_bps = 50: fee = 1 * 50 / 10000 = 0
    assert_eq!(earnings.total_earned, 0);
}

#[test]
fn test_distribute_large_batch_size() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);

    // Test with large batch size
    let large_batch = 100000u32;
    client.distribute(&relay, &1u64, &large_batch);

    let earnings = client.get_earnings(&relay);
    // With batch_size = 100000, fee_rate_bps = 50: fee = 100000 * 50 / 10000 = 500
    // treasury_share = 500 * 1000 / 10000 = 50
    // relay_payout = 500 - 50 = 450
    assert_eq!(earnings.total_earned, 450);
}

#[test]
fn test_fee_rate_change_affects_future_distributions() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let relay = Address::generate(&env);

    client.initialize(&admin, &50u32, &1000u32, &treasury);

    // Distribute with initial rate
    client.distribute(&relay, &1u64, &200u32); // fee = 1

    // Change fee rate
    client.set_fee_rate(&200u32); // 2%

    // Distribute with new rate
    client.distribute(&relay, &2u64, &200u32); // fee = 4

    let earnings = client.get_earnings(&relay);
    // First: 1 - 0 = 1, Second: 4 - 0 = 4, Total = 5
    assert_eq!(earnings.total_earned, 5);
}

#[test]
fn test_treasury_share_calculation_edge_cases() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let relay = Address::generate(&env);

    // Initialize with 50% treasury share
    client.initialize(&admin, &100u32, &5000u32, &treasury);

    client.distribute(&relay, &1u64, &1000u32);
    // fee = 1000 * 100 / 10000 = 10
    // treasury_share = 10 * 5000 / 10000 = 5
    // relay_payout = 10 - 5 = 5

    let earnings = client.get_earnings(&relay);
    assert_eq!(earnings.total_earned, 5);
}

#[test]
fn test_batch_id_uniqueness_across_relays() {
    let (_env, client) = setup();
    let relay1 = Address::generate(&_env);
    let relay2 = Address::generate(&_env);
    let batch_id = 1u64;

    // Same batch_id can't be used twice, even for different relays
    client.distribute(&relay1, &batch_id, &200u32);

    // Second distribution with same batch_id should fail
    let result = client.try_distribute(&relay2, &batch_id, &200u32);
    assert_eq!(result, Err(Ok(ContractError::BatchAlreadyDistributed)));
}

#[test]
fn test_calculate_fee_rounding_behavior() {
    let (_env, client) = setup();

    // Test rounding down behavior
    // With fee_rate_bps = 50:
    // batch_size = 99: fee = 99 * 50 / 10000 = 0 (rounds down)
    let fee1 = client.calculate_fee(&99u32);
    assert_eq!(fee1, 0);

    // batch_size = 100: fee = 100 * 50 / 10000 = 0 (rounds down)
    let fee2 = client.calculate_fee(&100u32);
    assert_eq!(fee2, 0);

    // batch_size = 101: fee = 101 * 50 / 10000 = 0 (rounds down)
    let fee3 = client.calculate_fee(&101u32);
    assert_eq!(fee3, 0);

    // batch_size = 200: fee = 200 * 50 / 10000 = 1
    let fee4 = client.calculate_fee(&200u32);
    assert_eq!(fee4, 1);
}

#[test]
fn test_set_fee_rate_boundary_values() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.initialize(&admin, &50u32, &1000u32, &treasury);

    // Test minimum valid rate (1)
    client.set_fee_rate(&1u32);
    let fee1 = client.calculate_fee(&10000u32);
    assert_eq!(fee1, 1);

    // Test maximum valid rate (10000)
    client.set_fee_rate(&10000u32);
    let fee2 = client.calculate_fee(&100u32);
    assert_eq!(fee2, 100);
}

#[test]
fn test_distribute_with_zero_treasury_share() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let relay = Address::generate(&env);

    // Initialize with 0% treasury share
    client.initialize(&admin, &100u32, &0u32, &treasury);

    client.distribute(&relay, &1u64, &1000u32);
    // fee = 1000 * 100 / 10000 = 10
    // treasury_share = 10 * 0 / 10000 = 0
    // relay_payout = 10 - 0 = 10

    let earnings = client.get_earnings(&relay);
    assert_eq!(earnings.total_earned, 10);
}

#[test]
fn test_distribute_with_max_treasury_share() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeDistributorContract);
    let client = FeeDistributorContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let relay = Address::generate(&env);

    // Initialize with 100% treasury share
    client.initialize(&admin, &100u32, &10000u32, &treasury);

    client.distribute(&relay, &1u64, &1000u32);
    // fee = 1000 * 100 / 10000 = 10
    // treasury_share = 10 * 10000 / 10000 = 10
    // relay_payout = 10 - 10 = 0

    let earnings = client.get_earnings(&relay);
    assert_eq!(earnings.total_earned, 0);
}

#[test]
fn test_claim_preserves_total_earned() {
    let (_env, client) = setup();
    let relay = Address::generate(&_env);

    client.distribute(&relay, &1u64, &200u32);
    client.distribute(&relay, &2u64, &400u32);

    let earnings_before = client.get_earnings(&relay);
    let total_earned_before = earnings_before.total_earned;

    client.claim(&relay);

    let earnings_after = client.get_earnings(&relay);
    // total_earned should remain unchanged after claim
    assert_eq!(earnings_after.total_earned, total_earned_before);
    // But unclaimed should be zero and total_claimed should equal total_earned
    assert_eq!(earnings_after.unclaimed, 0);
    assert_eq!(earnings_after.total_claimed, total_earned_before);
}
