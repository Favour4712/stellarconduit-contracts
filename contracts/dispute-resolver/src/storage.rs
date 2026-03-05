//! # Dispute Resolver Contract — `storage.rs`
//!
//! Provides typed helper functions for reading and writing persistent contract
//! storage using Soroban's `Env::storage()` API.
//!
//! ## Storage keys to implement
//! - `DataKey::Dispute(u64)` — Stores a `Dispute` keyed by dispute_id
//! - `DataKey::Ruling(u64)` — Stores a `Ruling` keyed by dispute_id
//! - `DataKey::DisputeCount` — Monotonically incrementing dispute ID counter
//! - `DataKey::ResolutionWindow` — Number of ledgers allowed for dispute response
//! - `DataKey::Admin` — Address authorized to configure the contract
//!
//! ## Functions to implement
//! - `get_dispute(env, dispute_id) -> Option<Dispute>` — Load a dispute from storage
//! - `set_dispute(env, dispute_id, dispute)` — Persist a dispute to storage
//! - `get_ruling(env, dispute_id) -> Option<Ruling>` — Load a ruling from storage
//! - `set_ruling(env, dispute_id, ruling)` — Persist a ruling to storage
//! - `next_dispute_id(env) -> u64` — Atomically increment and return the next dispute ID
//! - `get_resolution_window(env) -> u32` — Load the resolution window in ledgers
//!
//! implementation tracked in GitHub issue

use soroban_sdk::{contracttype, Address, Env};

use crate::types::{Dispute, Ruling};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Dispute(u64),
    DisputeCount,
    Ruling(u64),
    ResolutionWindow,
    Admin,
}

/// Load a dispute by its ID. Returns None if not found.
pub fn get_dispute(env: &Env, dispute_id: u64) -> Option<Dispute> {
    env.storage()
        .persistent()
        .get(&DataKey::Dispute(dispute_id))
}

/// Persist an updated dispute record to storage.
pub fn set_dispute(env: &Env, dispute_id: u64, dispute: &Dispute) {
    env.storage()
        .persistent()
        .set(&DataKey::Dispute(dispute_id), dispute);
}

/// Get the current dispute count. Returns 0 if none exist.
pub fn get_dispute_count(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::DisputeCount)
        .unwrap_or(0)
}

/// Increment and return the next available dispute ID.
pub fn get_next_dispute_id(env: &Env) -> u64 {
    let mut count = get_dispute_count(env);
    count = count.checked_add(1).expect("dispute count overflow");
    env.storage().instance().set(&DataKey::DisputeCount, &count);
    count
}

/// Load a ruling by the dispute ID. Returns None if no ruling exists.
pub fn get_ruling(env: &Env, dispute_id: u64) -> Option<Ruling> {
    env.storage().persistent().get(&DataKey::Ruling(dispute_id))
}

/// Persist a final ruling to storage.
pub fn set_ruling(env: &Env, dispute_id: u64, ruling: &Ruling) {
    env.storage()
        .persistent()
        .set(&DataKey::Ruling(dispute_id), ruling);
}

/// Get the resolution window in ledgers.
pub fn get_resolution_window(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::ResolutionWindow)
        .unwrap_or(0)
}

/// Set the resolution window in ledgers.
pub fn set_resolution_window(env: &Env, window_ledgers: u32) {
    env.storage()
        .instance()
        .set(&DataKey::ResolutionWindow, &window_ledgers);
}

/// Get the admin address.
pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("admin not set")
}

/// Set the admin address.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}
