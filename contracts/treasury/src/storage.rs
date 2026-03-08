//! # Treasury Contract — `storage.rs`
//!
//! Provides typed helper functions for reading and writing persistent contract
//! storage using Soroban's `Env::storage()` API.
//!
//! ## Storage keys to implement
//! - `DataKey::Balance` — Current treasury token balance (i128)
//! - `DataKey::EntryCount` — Total number of recorded treasury entries
//! - `DataKey::Entry(u64)` — A `TreasuryEntry` keyed by entry_id
//! - `DataKey::Allocation(String)` — An `AllocationRecord` keyed by program name
//! - `DataKey::Admin` — Address authorized to perform withdrawals and allocations
//! - `DataKey::TokenAddress` — The SAC (Stellar Asset Contract) address for the treasury token
//!
//! ## Functions to implement
//! - `get_balance(env) -> i128` — Load the current treasury balance
//! - `set_balance(env, balance)` — Persist an updated balance
//! - `get_entry(env, entry_id) -> Option<TreasuryEntry>` — Load a specific history entry
//! - `append_entry(env, entry)` — Append a new entry and increment the entry counter
//! - `get_entry_count(env) -> u64` — Return total number of entries in history
//! - `get_allocation(env, program) -> Option<AllocationRecord>` — Load an allocation record
//! - `set_allocation(env, program, record)` — Persist an allocation record
//! - `get_admin(env) -> Address` — Load the treasury admin address
//! - `get_token_address(env) -> Address` — Load the treasury token SAC address
//!
//! implementation tracked in GitHub issue

use soroban_sdk::{contracttype, Address, Env, String};

use crate::types::{AdminCouncil, AllocationRecord, SpendingProgram, TreasuryEntry, TreasuryStats};

/// Storage keys for the treasury contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Current treasury token balance (i128).
    Balance,
    /// Total number of recorded treasury entries.
    EntryCount,
    /// A TreasuryEntry keyed by entry_id.
    Entry(u64),
    /// Allocation records keyed by program name.
    Allocation(String),
    /// A SpendingProgram keyed by program_id.
    SpendingProgram(u64),
    /// Council authorized to perform withdrawals and allocations.
    AdminCouncil,
    /// The SAC (Stellar Asset Contract) address for the treasury token.
    TokenAddress,
    /// Aggregate treasury statistics for dashboard integration.
    Stats,
}

pub fn get_balance(env: &Env) -> i128 {
    env.storage().instance().get(&DataKey::Balance).unwrap_or(0)
}

pub fn set_balance(env: &Env, balance: i128) {
    env.storage().instance().set(&DataKey::Balance, &balance);
}

pub fn get_entry(env: &Env, entry_id: u64) -> Option<TreasuryEntry> {
    env.storage().persistent().get(&DataKey::Entry(entry_id))
}

/// Append a new entry and increment the entry counter.
pub fn set_entry(env: &Env, entry: TreasuryEntry) {
    let count = get_entry_count(env);
    let next_id = count + 1;
    env.storage()
        .persistent()
        .set(&DataKey::Entry(next_id), &entry);
    env.storage().instance().set(&DataKey::EntryCount, &next_id);
}

/// Return total number of entries in history.
pub fn get_entry_count(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::EntryCount)
        .unwrap_or(0)
}

pub fn set_entry_count(env: &Env, count: u64) {
    env.storage().instance().set(&DataKey::EntryCount, &count);
}

pub fn append_entry(env: &Env, entry: &TreasuryEntry) {
    let next_id = get_entry_count(env)
        .checked_add(1)
        .expect("entry count overflow");
    env.storage()
        .persistent()
        .set(&DataKey::Entry(next_id), entry);
    set_entry_count(env, next_id);
}

pub fn get_allocation(env: &Env, program: &String) -> Option<AllocationRecord> {
    env.storage()
        .persistent()
        .get(&DataKey::Allocation(program.clone()))
}

pub fn set_allocation(env: &Env, program: &String, record: &AllocationRecord) {
    env.storage()
        .persistent()
        .set(&DataKey::Allocation(program.clone()), record);
}

/// Load a spending program by ID.
pub fn get_spending_program(env: &Env, program_id: u64) -> Option<SpendingProgram> {
    env.storage()
        .persistent()
        .get(&DataKey::SpendingProgram(program_id))
}

/// Persist a spending program.
pub fn set_spending_program(env: &Env, program_id: u64, program: SpendingProgram) {
    env.storage()
        .persistent()
        .set(&DataKey::SpendingProgram(program_id), &program);
}

/// Load the treasury admin council.
pub fn get_admin_council(env: &Env) -> AdminCouncil {
    env.storage()
        .instance()
        .get(&DataKey::AdminCouncil)
        .expect("admin council not initialized")
}

/// Set the treasury admin council.
pub fn set_admin_council(env: &Env, council: &AdminCouncil) {
    env.storage()
        .instance()
        .set(&DataKey::AdminCouncil, council);
}

/// Check if the admin council is set.
pub fn has_admin_council(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::AdminCouncil)
}

/// Load the treasury token SAC address.
pub fn get_token_address(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::TokenAddress)
        .expect("token address not initialized")
}

/// Set the treasury token SAC address.
pub fn set_token_address(env: &Env, token_address: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::TokenAddress, token_address);
}

/// Load the treasury statistics. Returns default zeros if not found.
pub fn get_stats(env: &Env) -> TreasuryStats {
    env.storage()
        .instance()
        .get(&DataKey::Stats)
        .unwrap_or(TreasuryStats {
            current_balance: 0,
            lifetime_deposited: 0,
            lifetime_withdrawn: 0,
            lifetime_allocated: 0,
        })
}

/// Persist the treasury statistics.
pub fn set_stats(env: &Env, stats: &TreasuryStats) {
    env.storage().instance().set(&DataKey::Stats, stats);
}
