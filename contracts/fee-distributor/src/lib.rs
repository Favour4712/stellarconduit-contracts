//! # Fee Distributor Contract — `lib.rs`
//!
//! This is the main entry point for the Fee Distributor Soroban smart contract.
//! It exposes the public contract interface for protocol fee calculation and
//! distribution to relay nodes upon successful transaction settlement.
//!
//! ## Responsibilities
//! - Calculate relay fee based on batch size and transaction count
//! - Distribute fees to relay nodes upon confirmed settlement on Stellar
//! - Allocate a protocol treasury share from collected fees
//! - Track cumulative fee earnings per relay node
//! - Handle delayed fee claims for relay nodes
//!
//! ## Functions to implement
//! - `distribute(env, relay_address, batch_id)` — Distribute fee for a settled transaction batch
//! - `calculate_fee(env, batch_size)` — Calculate the fee for a given batch of transactions
//! - `claim(env, relay_address)` — Claim accumulated, unclaimed fees for a relay node
//! - `get_earnings(env, relay_address)` — View total lifetime earnings for a relay node
//! - `set_fee_rate(env, rate)` — Update the protocol fee rate (governance-only)
//!
//! ## See also
//! - `types.rs` — Data structures (FeeEntry, EarningsRecord, FeeConfig)
//! - `storage.rs` — Persistent storage helpers
//! - `errors.rs` — Contract error codes
//!
//! implementation tracked in GitHub issue

#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env};

pub mod errors;
pub mod storage;
pub mod types;

use crate::errors::ContractError;

#[contract]
pub struct FeeDistributorContract;

#[contractimpl]
impl FeeDistributorContract {
    /// Calculate the total fee for a given batch of transactions.
    ///
    /// This is a pure calculation function that reads the configured fee rate
    /// and returns the total fee amount. No storage is written.
    ///
    /// # Formula
    /// `fee = (batch_size as i128) * (fee_rate_bps as i128) / 10000`
    ///
    /// # Example
    /// - With `fee_rate_bps = 50` (0.5%) and `batch_size = 200`:
    ///   `fee = 200 * 50 / 10000 = 1`
    /// - With `fee_rate_bps = 500` (5%) and `batch_size = 1000`:
    ///   `fee = 1000 * 500 / 10000 = 50`
    ///
    /// # Parameters
    /// - `env`: Soroban environment.
    /// - `batch_size`: Number of transactions in the settled batch.
    ///
    /// # Errors
    /// - `ContractError::InvalidBatchSize` if `batch_size` is zero.
    /// - `ContractError::Overflow` if the calculation overflows.
    pub fn calculate_fee(env: Env, batch_size: u32) -> Result<i128, ContractError> {
        if batch_size == 0 {
            return Err(ContractError::InvalidBatchSize);
        }

        let config = storage::get_fee_config(&env);

        let total = (batch_size as i128)
            .checked_mul(config.fee_rate_bps as i128)
            .ok_or(ContractError::Overflow)?;

        let fee = total.checked_div(10000).ok_or(ContractError::Overflow)?;

        Ok(fee)
    }

    /// Distribute the fee for a successfully settled transaction batch.
    ///
    /// This function calculates the fee, credits the relay node's earnings,
    /// allocates the protocol treasury share, and permanently records the
    /// distribution event.
    ///
    /// # Parameters
    /// - `env`: Soroban environment.
    /// - `relay_address`: Address of the relay node that settled the batch.
    /// - `batch_id`: Unique identifier of the settled transaction batch.
    /// - `batch_size`: Number of transactions in the batch.
    ///
    /// # Errors
    /// - `ContractError::BatchAlreadyDistributed` if `batch_id` has already been processed.
    /// - `ContractError::InvalidBatchSize` if `batch_size` is zero.
    /// - `ContractError::Overflow` if fee/split calculation overflows.
    pub fn distribute(
        env: Env,
        relay_address: Address,
        batch_id: u64,
        batch_size: u32,
    ) -> Result<(), ContractError> {
        if storage::get_fee_entry(&env, batch_id).is_some() {
            return Err(ContractError::BatchAlreadyDistributed);
        }

        let fee = Self::calculate_fee(env.clone(), batch_size)?;
        let config = storage::get_fee_config(&env);

        let treasury_share = fee
            .checked_mul(config.treasury_share_bps as i128)
            .ok_or(ContractError::Overflow)?
            .checked_div(10000)
            .ok_or(ContractError::Overflow)?;

        let relay_payout = fee
            .checked_sub(treasury_share)
            .ok_or(ContractError::Overflow)?;

        let mut record = storage::get_earnings(&env, &relay_address);
        record.total_earned = record
            .total_earned
            .checked_add(relay_payout)
            .ok_or(ContractError::Overflow)?;
        record.unclaimed = record
            .unclaimed
            .checked_add(relay_payout)
            .ok_or(ContractError::Overflow)?;

        storage::set_earnings(&env, &relay_address, &record);

        let entry = crate::types::FeeEntry {
            batch_id,
            relay_address: relay_address.clone(),
            amount: fee,
            treasury_share,
            settled_at: env.ledger().timestamp(),
        };
        storage::set_fee_entry(&env, batch_id, &entry);

        env.events().publish(
            ("distribute",),
            (relay_address.clone(), batch_id, relay_payout),
        );

        // TODO: SAC transfer treasury_share to treasury
        Ok(())
    }

    /// Claim accumulated, unclaimed fees for a relay node.
    ///
    /// This allows a relay node to withdraw all its accumulated, unclaimed fees
    /// to its own address. Upon successful claim, the unclaimed balance is reset
    /// to zero, and the total claimed amount is increased.
    ///
    /// # Parameters
    /// - `env`: Soroban environment.
    /// - `relay_address`: Address of the relay node claiming its fees. Must authorize the call.
    ///
    /// # Returns
    /// The total amount of fees claimed in this transaction.
    ///
    /// # Errors
    /// - `ContractError::NothingToClaim` if the relay node has no unclaimed earnings.
    /// - `ContractError::Overflow` if the arithmetic for updating `total_claimed` overflows.
    pub fn claim(env: Env, relay_address: Address) -> Result<i128, ContractError> {
        relay_address.require_auth();

        let mut record = storage::get_earnings(&env, &relay_address);

        if record.unclaimed == 0 {
            return Err(ContractError::NothingToClaim);
        }

        let payout = record.unclaimed;

        record.total_claimed = record
            .total_claimed
            .checked_add(payout)
            .ok_or(ContractError::Overflow)?;
        record.unclaimed = 0;

        storage::set_earnings(&env, &relay_address, &record);

        env.events()
            .publish(("claim",), (relay_address.clone(), payout));

        // TODO: SAC transfer payout to relay_address
        Ok(payout)
    }
}
