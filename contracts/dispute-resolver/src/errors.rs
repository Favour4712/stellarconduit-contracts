//! # Dispute Resolver Contract — `errors.rs`
//!
//! Defines all error codes returned by the Dispute Resolver contract.
//! All errors are exposed as a `ContractError` enum that maps to Soroban
//! `contracterror` integer values consumable by clients.

use soroban_sdk::contracterror;

/// Contract error codes returned by the Dispute Resolver.
#[contracterror]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractError {
    /// The specified dispute_id does not exist in storage.
    DisputeNotFound = 1,
    /// The dispute has already been resolved or ruled on.
    DisputeAlreadyResolved = 2,
    /// The dispute passed its resolution deadline without a response.
    DisputeExpired = 3,
    /// The dispute is still open and awaiting a counter-proof.
    DisputeNotResolvable = 4,
    /// The calling party has already submitted a proof for this dispute.
    ProofAlreadySubmitted = 5,
    /// The submitted relay chain proof fails cryptographic verification.
    InvalidProof = 6,
    /// Caller is not a party to this dispute.
    Unauthorized = 7,
    /// A dispute for this transaction ID already exists.
    DuplicateDispute = 8,
    /// Arithmetic overflow in dispute ID generation.
    Overflow = 9,
    /// The dispute is not in Open status.
    NotOpen = 10,
    /// The resolution window has expired.
    ResolutionWindowExpired = 11,
    /// The resolution window is still active.
    ResolutionWindowActive = 12,
    /// The dispute has not been responded to.
    NotResponded = 13,
    /// The contract has already been initialized.
    AlreadyInitialized = 14,
    /// Invalid configuration parameter.
    InvalidConfig = 15,
}
