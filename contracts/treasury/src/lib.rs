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

use soroban_sdk::{contract, contractimpl, Address, Env, String};

pub mod errors;
pub mod storage;
pub mod types;

use crate::errors::ContractError;
use crate::types::{EntryKind, TreasuryEntry};

#[contract]
pub struct TreasuryContract;

#[contractimpl]
impl TreasuryContract {
    /// Deposit funds into the protocol treasury.
    ///
    /// # Parameters
    /// - `env`: Soroban environment for the current invocation.
    /// - `from`: Address funding the deposit. Must authorize this call.
    /// - `amount`: Amount to deposit. Must be greater than zero.
    ///
    /// # Errors
    /// - `ContractError::InvalidAmount` if `amount` is zero or negative.
    /// - `ContractError::Overflow` if the balance arithmetic overflows.
    pub fn deposit(env: Env, from: Address, amount: i128) -> Result<(), ContractError> {
        from.require_auth();

        if amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        let balance = storage::get_balance(&env);
        let new_balance = balance.checked_add(amount).ok_or(ContractError::Overflow)?;
        storage::set_balance(&env, new_balance);

        let entry = TreasuryEntry {
            kind: EntryKind::Deposit,
            amount,
            actor: from.clone(),
            recipient: None,
            memo: String::from_str(&env, "deposit"),
            ledger: env.ledger().sequence() as u64,
        };
        storage::set_entry(&env, entry);

        // TODO: SAC transfer

        env.events().publish(("deposit",), (from.clone(), amount));

        Ok(())
    }

    /// Withdraw funds from the protocol treasury (admin only).
    ///
    /// # Parameters
    /// - `env`: Soroban environment for the current invocation.
    /// - `to`: Recipient of the withdrawal.
    /// - `amount`: Amount to withdraw. Must be greater than zero.
    /// - `memo`: Human-readable memo for the withdrawal entry.
    ///
    /// # Errors
    /// - `ContractError::InvalidAmount` if `amount` is zero or negative.
    /// - `ContractError::InsufficientBalance` if treasury balance is too low.
    /// - `ContractError::Overflow` if arithmetic underflows/overflows.
    pub fn withdraw(
        env: Env,
        to: Address,
        amount: i128,
        memo: String,
    ) -> Result<(), ContractError> {
        let admin = storage::get_admin(&env);
        admin.require_auth();

        if amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        let balance = storage::get_balance(&env);
        if balance < amount {
            return Err(ContractError::InsufficientBalance);
        }

        let new_balance = balance.checked_sub(amount).ok_or(ContractError::Overflow)?;
        storage::set_balance(&env, new_balance);

        let entry = TreasuryEntry {
            kind: EntryKind::Withdrawal,
            amount,
            actor: admin.clone(),
            recipient: Some(to.clone()),
            memo,
            ledger: env.ledger().sequence() as u64,
        };
        storage::set_entry(&env, entry);

        env.events().publish(("withdraw",), (to.clone(), amount));

        Ok(())
    }

    /// Allocate treasury funds to a spending program (admin only).
    ///
    /// # Parameters
    /// - `env`: Soroban environment for the current invocation.
    /// - `program_id`: ID of the spending program to allocate to.
    /// - `amount`: Amount to allocate. Must be greater than zero.
    ///
    /// # Errors
    /// - `ContractError::InvalidAmount` if `amount` is zero or negative.
    /// - `ContractError::ProgramNotFound` if the program does not exist.
    /// - `ContractError::ProgramInactive` if the program is not active.
    /// - `ContractError::ProgramOverBudget` if the allocation exceeds budget.
    /// - `ContractError::InsufficientBalance` if treasury balance is too low.
    /// - `ContractError::Overflow` if arithmetic overflows.
    pub fn allocate(env: Env, program_id: u64, amount: i128) -> Result<(), ContractError> {
        let admin = storage::get_admin(&env);
        admin.require_auth();

        if amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        let mut program = storage::get_spending_program(&env, program_id)
            .ok_or(ContractError::ProgramNotFound)?;

        if !program.active {
            return Err(ContractError::ProgramInactive);
        }

        let new_spent = program
            .spent
            .checked_add(amount)
            .ok_or(ContractError::Overflow)?;
        if new_spent > program.budget {
            return Err(ContractError::ProgramOverBudget);
        }

        let balance = storage::get_balance(&env);
        if balance < amount {
            return Err(ContractError::InsufficientBalance);
        }

        program.spent = new_spent;
        storage::set_spending_program(&env, program_id, program);

        let new_balance = balance.checked_sub(amount).ok_or(ContractError::Overflow)?;
        storage::set_balance(&env, new_balance);

        let entry = TreasuryEntry {
            kind: EntryKind::Allocation,
            amount,
            actor: admin,
            recipient: None,
            memo: String::from_str(&env, "allocation"),
            ledger: env.ledger().sequence() as u64,
        };
        storage::set_entry(&env, entry);

        Ok(())
    }
}

#[cfg(test)]
mod test;
