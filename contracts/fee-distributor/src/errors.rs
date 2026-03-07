//! # Fee Distributor Contract — `errors.rs`
//!
//! Defines all error codes returned by the Fee Distributor contract.
//! All errors are exposed as a `ContractError` enum that maps to Soroban
//! `contracterror` integer values consumable by clients.

use soroban_sdk::contracterror;

/// All error codes returned by the Fee Distributor contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// Fee for this batch_id has already been distributed.
    BatchAlreadyDistributed = 1,

    /// The specified batch_id does not exist.
    BatchNotFound = 2,

    /// The relay node has no unclaimed earnings.
    NothingToClaim = 3,

    /// Fee rate is outside of the allowed range.
    InvalidFeeRate = 4,

    /// Caller is not authorized to perform this action (e.g., set_fee_rate).
    Unauthorized = 5,

    /// Batch size is zero or exceeds the maximum.
    InvalidBatchSize = 6,

    /// Token transfer to treasury address failed.
    TreasuryTransferFailed = 7,

    /// Arithmetic overflow in fee calculation.
    Overflow = 8,

    /// Contract has already been initialized.
    AlreadyInitialized = 9,
}
