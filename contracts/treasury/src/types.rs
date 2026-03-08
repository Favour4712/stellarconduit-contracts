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

use soroban_sdk::{contracttype, Address, String, Vec};

/// A multi-signature admin council requiring threshold approvals.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdminCouncil {
    /// List of council member addresses (max 10)
    pub members: Vec<Address>,
    /// Minimum number of members required to authorize a sensitive action
    pub threshold: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllocationRecord {
    pub program: String,
    pub allocated: i128,
    pub spent: i128,
}

/// Kind of treasury transaction entry.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// Deposit into the treasury.
    Deposit,
    /// Withdrawal from the treasury.
    Withdrawal,
    /// Allocation to a spending program.
    Allocation,
}

/// A record of a single treasury transaction (deposit, withdrawal, or allocation).
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreasuryEntry {
    /// Type of transaction: Deposit, Withdrawal, or Allocation.
    pub kind: EntryKind,
    /// Token amount of the transaction.
    pub amount: i128,
    /// Address that initiated the transaction.
    pub actor: Address,
    /// Recipient address for withdrawals (None for deposits/allocations).
    pub recipient: Option<Address>,
    /// Human-readable reason or memo.
    pub memo: String,
    /// Ledger number when the entry occurred.
    pub ledger: u64,
}

/// A spending program with budget tracking.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpendingProgram {
    /// Unique program ID.
    pub program_id: u64,
    /// Total budget allocated to this program.
    pub budget: i128,
    /// Amount already spent from this program.
    pub spent: i128,
    /// Whether the program is currently active.
    pub active: bool,
    /// Human-readable name/description.
    pub name: String,
}

/// Aggregate statistics for the treasury, used for dashboard integration.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreasuryStats {
    /// Current treasury token balance.
    pub current_balance: i128,
    /// Total tokens deposited over the treasury's lifetime.
    pub lifetime_deposited: i128,
    /// Total tokens withdrawn over the treasury's lifetime.
    pub lifetime_withdrawn: i128,
    /// Total tokens allocated to spending programs over the treasury's lifetime.
    pub lifetime_allocated: i128,
}
