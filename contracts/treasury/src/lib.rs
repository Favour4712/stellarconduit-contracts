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

use soroban_sdk::{contract, contractimpl, token, Address, Env, String};

pub mod errors;
pub mod storage;
pub mod types;

use crate::errors::ContractError;
use crate::types::{AdminCouncil, EntryKind, TreasuryEntry, TreasuryStats};

fn require_council_auth(env: &Env) {
    let council = storage::get_admin_council(env);
    let mut authorized = 0u32;
    for member in council.members.iter() {
        member.require_auth();
        authorized += 1;
        if authorized >= council.threshold {
            break;
        }
    }

    if authorized < council.threshold {
        panic!("Insufficient approvals");
    }
}

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
        council: AdminCouncil,
        token_address: Address,
    ) -> Result<(), ContractError> {
        if storage::has_admin_council(&env) {
            return Err(ContractError::AlreadyInitialized);
        }

        if council.threshold == 0 || council.members.len() < council.threshold {
            return Err(ContractError::InvalidCouncilConfig);
        }

        storage::set_admin_council(&env, &council);
        storage::set_token_address(&env, &token_address);
        storage::set_balance(&env, 0);

        Ok(())
    }

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

        // Update lifetime stats
        let mut stats = storage::get_stats(&env);
        stats.lifetime_deposited = stats
            .lifetime_deposited
            .checked_add(amount)
            .ok_or(ContractError::Overflow)?;
        storage::set_stats(&env, &stats);

        let entry = TreasuryEntry {
            kind: EntryKind::Deposit,
            amount,
            actor: from.clone(),
            recipient: None,
            memo: String::from_str(&env, "deposit"),
            ledger: env.ledger().sequence() as u64,
        };
        storage::set_entry(&env, entry);

        let token_address = storage::get_token_address(&env);
        let token = token::Client::new(&env, &token_address);
        token.transfer(&from, &env.current_contract_address(), &amount);

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
        require_council_auth(&env);

        if amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        let balance = storage::get_balance(&env);
        if balance < amount {
            return Err(ContractError::InsufficientBalance);
        }

        let new_balance = balance.checked_sub(amount).ok_or(ContractError::Overflow)?;
        storage::set_balance(&env, new_balance);

        // Update lifetime stats
        let mut stats = storage::get_stats(&env);
        stats.lifetime_withdrawn = stats
            .lifetime_withdrawn
            .checked_add(amount)
            .ok_or(ContractError::Overflow)?;
        storage::set_stats(&env, &stats);

        let entry = TreasuryEntry {
            kind: EntryKind::Withdrawal,
            amount,
            actor: env.current_contract_address(), // We record the contract executing since it's a multisig operation
            recipient: Some(to.clone()),
            memo,
            ledger: env.ledger().sequence() as u64,
        };
        storage::set_entry(&env, entry);

        let token = token::Client::new(&env, &storage::get_token_address(&env));
        token.transfer(&env.current_contract_address(), &to, &amount);

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
        require_council_auth(&env);

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

        // Update lifetime stats
        let mut stats = storage::get_stats(&env);
        stats.lifetime_allocated = stats
            .lifetime_allocated
            .checked_add(amount)
            .ok_or(ContractError::Overflow)?;
        storage::set_stats(&env, &stats);

        let entry = TreasuryEntry {
            kind: EntryKind::Allocation,
            amount,
            actor: env.current_contract_address(), // We record the contract executing since it's a multisig operation
            recipient: None,
            memo: String::from_str(&env, "allocation"),
            ledger: env.ledger().sequence() as u64,
        };
        storage::set_entry(&env, entry);

        // Warning: Local program recipient address map missing. Treasury allocate usually
        // moves funds to a dedicated escrow account or recipient specified by the program.
        // For Issue #36: `transfer amount tokens out of the treasury to the to / program recipient address`
        // Given that `SpendingProgram` does not have a `recipient_address` defined in `types.rs`,
        // and its signature is `allocate(env, program, amount)`, we cannot transfer it correctly on-chain
        // without knowing `to`. A `TODO` is raised.

        // TODO: Map program_id to its recipient address and transfer SAC token.
        // let token = token::Client::new(&env, &storage::get_token_address(&env));
        // token.transfer(&env.current_contract_address(), &program_recipient_address, &amount);

        Ok(())
    }

    /// Returns aggregate statistics for the treasury.
    ///
    /// This is a read-only view function intended for dashboard integration.
    /// Returns cumulative totals for deposits, withdrawals, and allocations.
    ///
    /// # Returns
    /// - `TreasuryStats` struct containing:
    ///   - `current_balance`: Current treasury token balance
    ///   - `lifetime_deposited`: Total tokens deposited over the treasury's lifetime
    ///   - `lifetime_withdrawn`: Total tokens withdrawn over the treasury's lifetime
    ///   - `lifetime_allocated`: Total tokens allocated to spending programs
    pub fn get_treasury_stats(env: Env) -> TreasuryStats {
        let mut stats = storage::get_stats(&env);
        // current_balance is dynamic, fetch it fresh to ensure accuracy
        stats.current_balance = storage::get_balance(&env);
        stats
    }
}

#[cfg(test)]
mod test;
