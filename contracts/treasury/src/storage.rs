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

use crate::types::{AllocationRecord, TreasuryEntry};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Balance,
    EntryCount,
    Entry(u64),
    Allocation(String),
    Admin,
    TokenAddress,
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

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_token_address(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::TokenAddress)
}

pub fn set_token_address(env: &Env, addr: &Address) {
    env.storage().instance().set(&DataKey::TokenAddress, addr);
}
