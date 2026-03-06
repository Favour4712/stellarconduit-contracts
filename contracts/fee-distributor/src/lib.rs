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

use soroban_sdk::{contract, contractimpl, Env};

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
}
