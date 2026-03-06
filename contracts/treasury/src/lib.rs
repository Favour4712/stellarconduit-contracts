//! # Treasury Contract — `lib.rs`
//!
//! This is the main entry point for the Protocol Treasury Soroban smart contract.
//! The treasury holds protocol funds for relay node incentive programs, grants for
//! operators in underserved and remote regions, and ongoing protocol development.
//!
//! ## Responsibilities
//! - Receive fee allocations from the Fee Distributor contract
//! - Disburse grants and incentives to relay node operators
//! - Track all inflows and outflows with on-chain transparency
//! - Enforce spending limits and require multi-sig authorization for withdrawals
//! - Support future handover to a DAO governance model
//!
//! ## Functions to implement
//! - `deposit(env, amount)` — Deposit funds into the protocol treasury
//! - `withdraw(env, amount, recipient, reason)` — Withdraw funds (authorized callers only)
//! - `allocate(env, program, amount)` — Allocate budget to a named spending program
//! - `get_balance(env)` — Fetch the current treasury token balance
//! - `get_history(env)` — Fetch the full on-chain transaction history
//!
//! ## See also
//! - `types.rs` — Data structures (TreasuryEntry, AllocationRecord, SpendingProgram)
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
use crate::types::TreasuryEntry;

#[contract]
pub struct TreasuryContract;

#[contractimpl]
impl TreasuryContract {
    /// Returns the current treasury token balance.
    ///
    /// Public view function; never errors. Returns 0 if balance is unset.
    pub fn get_balance(env: Env) -> i128 {
        storage::get_balance(&env)
    }

    /// Returns a specific history entry by its ID for auditing.
    ///
    /// Uses `ContractError::ProgramNotFound` when an entry is not found.
    pub fn get_history(env: Env, entry_id: u64) -> Result<TreasuryEntry, ContractError> {
        storage::get_entry(&env, entry_id).ok_or(ContractError::ProgramNotFound)
    }

    /// One-time setup configuring the admin and token address.
    ///
    /// First caller wins; no auth required. Fails if already initialized.
    pub fn initialize(
        env: Env,
        admin: Address,
        token_address: Address,
    ) -> Result<(), ContractError> {
        if storage::get_admin(&env).is_some() {
            return Err(ContractError::AlreadyInitialized);
        }

        storage::set_admin(&env, &admin);
        storage::set_token_address(&env, &token_address);
        storage::set_balance(&env, 0);

        Ok(())
    }
}
