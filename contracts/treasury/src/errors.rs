//! # Treasury Contract — `errors.rs`
//!
//! Defines all error codes returned by the Protocol Treasury contract.
//! All errors are exposed as a `ContractError` enum that maps to Soroban
//! `contracterror` integer values consumable by clients.
//!
//! ## Error codes to implement
//! - `InsufficientBalance (1)` — Treasury balance is below the requested withdrawal amount
//! - `Unauthorized (2)` — Caller is not authorized (not admin or multi-sig signer)
//! - `InvalidAmount (3)` — Withdrawal or deposit amount is zero or negative
//! - `ProgramNotFound (4)` — Specified spending program has no allocation record
//! - `AllocationExceeded (5)` — Withdrawal would exceed the program's remaining allocation
//! - `InvalidRecipient (6)` — Recipient address fails validation
//! - `TokenTransferFailed (7)` — SAC token transfer call failed
//! - `HistoryOverflow (8)` — Entry ID counter has overflowed (unreachable in practice)
//! - `Overflow (9)` — Arithmetic overflow in balance arithmetic
//!
//! implementation tracked in GitHub issue

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    InsufficientBalance = 1,
    Unauthorized = 2,
    InvalidAmount = 3,
    ProgramNotFound = 4,
    AllocationExceeded = 5,
    InvalidRecipient = 6,
    TokenTransferFailed = 7,
    HistoryOverflow = 8,
    Overflow = 9,
    AlreadyInitialized = 10,
}
