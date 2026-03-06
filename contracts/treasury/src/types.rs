//! # Treasury Contract — `types.rs`
//!
//! Defines all data structures used by the Protocol Treasury contract.
//!
//! ## Types to implement
//! - `TreasuryEntry` — A record of a single treasury transaction (deposit or withdrawal):
//!   - `entry_id: u64` — Unique monotonically incrementing entry identifier
//!   - `kind: EntryKind` — Deposit or Withdrawal
//!   - `amount: i128` — Token amount of the transaction
//!   - `actor: Address` — Address that initiated the transaction
//!   - `recipient: Option<Address>` — Recipient address for withdrawals
//!   - `reason: String` — Human-readable reason (e.g., "relay node grant – west africa Q1")
//!   - `ledger: u64` — Ledger number when the entry occurred
//! - `EntryKind` — Enum: `Deposit`, `Withdrawal`, `Allocation`
//! - `AllocationRecord` — A budget allocation to a named spending program:
//!   - `program: String` — Name of the spending program
//!   - `allocated: i128` — Total tokens allocated to the program
//!   - `spent: i128` — Tokens already spent from this allocation
//! - `SpendingProgram` — Enum of known spending programs:
//!   - `RelayIncentives` — Incentive rewards for high-uptime relay nodes
//!   - `UnderservedGrants` — Grants for relay nodes in underserved regions
//!   - `ProtocolDevelopment` — Development and infrastructure expenses
//!   - `Custom(String)` — Governance-defined custom programs
//!
//! implementation tracked in GitHub issue

use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntryKind {
    Deposit,
    Withdrawal,
    Allocation,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryEntry {
    pub entry_id: u64,
    pub kind: EntryKind,
    pub amount: i128,
    pub actor: Address,
    pub recipient: Option<Address>,
    pub reason: String,
    pub ledger: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllocationRecord {
    pub program: String,
    pub allocated: i128,
    pub spent: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpendingProgram {
    RelayIncentives,
    UnderservedGrants,
    ProtocolDevelopment,
    Custom(String),
}
