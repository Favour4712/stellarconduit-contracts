//! # Dispute Resolver Contract — `lib.rs`
//!
//! This is the main entry point for the Dispute Resolver Soroban smart contract.
//! It handles final on-chain arbitration for double-spend conflicts that cannot be
//! resolved off-chain by the StellarConduit sync engine.
//!
//! ## Responsibilities
//! - Accept dispute submissions with cryptographic relay chain proofs
//! - Enforce dispute submission and response deadlines
//! - Evaluate competing cryptographic proofs deterministically
//! - Issue a final ruling and trigger appropriate fund recovery
//! - Penalize the relay node that submitted the invalid transaction
//!
//! ## Functions
//! - `raise_dispute(env, initiator, tx_id, proof)` — Submit a new dispute with a relay chain proof
//! - `respond(env, respondent, dispute_id, proof)` — Submit a counter-proof to an open dispute
//! - `resolve(env, dispute_id)` — Resolve a dispute after the evaluation period
//!
//! ## See also
//! - `types.rs` — Data structures (Dispute, DisputeStatus, Ruling, RelayChainProof)
//! - `storage.rs` — Persistent storage helpers
//! - `errors.rs` — Contract error codes
//!
//! implementation tracked in GitHub issue

#![no_std]

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String};

pub mod errors;
pub mod storage;
pub mod types;

use crate::errors::ContractError;
use crate::types::{Dispute, DisputeStatus, OptionalRelayChainProof, RelayChainProof, Ruling};

#[contract]
pub struct DisputeResolverContract;

#[contractimpl]
impl DisputeResolverContract {
    /// Submit a new dispute for a suspected double-spend, recording the initiator's
    /// cryptographic relay chain proof on-chain and setting a response deadline.
    ///
    /// # Parameters
    /// - `env`: Soroban environment for the current contract invocation.
    /// - `initiator`: Address of the party raising the dispute. Must authorize this call.
    /// - `tx_id`: The 32-byte Stellar transaction ID under dispute.
    /// - `proof`: The initiator's cryptographic relay chain proof.
    ///
    /// # Returns
    /// The newly assigned `dispute_id` (`u64`) for tracking the dispute.
    ///
    /// # Errors
    /// - `ContractError::DuplicateDispute` if a dispute for this `tx_id` already exists.
    pub fn raise_dispute(
        env: Env,
        initiator: Address,
        tx_id: BytesN<32>,
        proof: RelayChainProof,
    ) -> Result<u64, ContractError> {
        initiator.require_auth();

        // Guard against duplicate disputes for the same tx_id.
        if storage::get_dispute_by_tx(&env, &tx_id).is_some() {
            return Err(ContractError::DuplicateDispute);
        }

        // Auto-increment and get the next dispute ID (starts at 1).
        let dispute_id = storage::get_next_dispute_id(&env);

        // Compute the response deadline as a ledger sequence number.
        let resolution_window = storage::get_resolution_window(&env);
        let resolve_by = env.ledger().sequence() + resolution_window;

        let dispute = Dispute {
            dispute_id,
            tx_id: tx_id.clone(),
            initiator: initiator.clone(),
            respondent: None,
            initiator_proof: proof,
            respondent_proof: OptionalRelayChainProof::None,
            status: DisputeStatus::Open,
            raised_at: env.ledger().timestamp(),
            resolve_by: resolve_by as u64,
        };

        // Persist the dispute and record the tx → dispute_id mapping.
        storage::set_dispute(&env, dispute_id, &dispute);
        storage::set_dispute_by_tx(&env, &tx_id, dispute_id);

        // Emit event for off-chain indexers.
        env.events()
            .publish(("raise_dispute",), (initiator, dispute_id, tx_id));

        Ok(dispute_id)
    }

    /// Submit a counter-proof to an open dispute within the resolution window.
    ///
    /// # Parameters
    /// - `env`: Soroban environment for the current contract invocation.
    /// - `respondent`: Address of the party responding to the dispute. Must authorize.
    /// - `dispute_id`: The unique ID of the dispute to respond to.
    /// - `proof`: The respondent's cryptographic relay chain proof.
    ///
    /// # Errors
    /// - `ContractError::DisputeNotFound` if no dispute exists for this ID.
    /// - `ContractError::NotOpen` if the dispute is not in `Open` status.
    /// - `ContractError::ResolutionWindowExpired` if the response deadline has passed.
    pub fn respond(
        env: Env,
        respondent: Address,
        dispute_id: u64,
        proof: RelayChainProof,
    ) -> Result<(), ContractError> {
        respondent.require_auth();

        let mut dispute =
            storage::get_dispute(&env, dispute_id).ok_or(ContractError::DisputeNotFound)?;

        // Only Open disputes can receive a response.
        if dispute.status != DisputeStatus::Open {
            return Err(ContractError::NotOpen);
        }

        // The response window must not have expired.
        if env.ledger().sequence() as u64 > dispute.resolve_by {
            return Err(ContractError::ResolutionWindowExpired);
        }

        dispute.respondent = Some(respondent.clone());
        dispute.respondent_proof = OptionalRelayChainProof::Some(proof);
        dispute.status = DisputeStatus::Responded;

        storage::set_dispute(&env, dispute_id, &dispute);

        env.events().publish(("respond",), (respondent, dispute_id));

        Ok(())
    }

    /// Evaluate both proofs and issue a final ruling for a dispute.
    ///
    /// Can be called by anyone once the dispute is in `Responded` status, or by
    /// anyone after the resolution window expires (ruling goes to initiator by default).
    ///
    /// # Parameters
    /// - `env`: Soroban environment for the current contract invocation.
    /// - `dispute_id`: The unique ID of the dispute to resolve.
    ///
    /// # Returns
    /// The final `Ruling` struct.
    ///
    /// # Errors
    /// - `ContractError::DisputeNotFound` if no dispute exists for this ID.
    /// - `ContractError::AlreadyResolved` if the dispute is already resolved.
    /// - `ContractError::ResolutionWindowActive` if the dispute is still Open and the window hasn't expired.
    /// - `ContractError::NotResponded` if the dispute status is unexpected.
    pub fn resolve(env: Env, dispute_id: u64) -> Result<Ruling, ContractError> {
        let mut dispute =
            storage::get_dispute(&env, dispute_id).ok_or(ContractError::DisputeNotFound)?;

        // Cannot resolve an already-resolved dispute.
        if dispute.status == DisputeStatus::Resolved {
            return Err(ContractError::AlreadyResolved);
        }

        let current_sequence = env.ledger().sequence() as u64;

        // If still Open, check if the window has expired. If not, cannot resolve yet.
        if dispute.status == DisputeStatus::Open {
            if current_sequence <= dispute.resolve_by {
                return Err(ContractError::ResolutionWindowActive);
            }
            // Window expired with no response — initiator wins automatically.
            let ruling = Ruling {
                dispute_id,
                winner: dispute.initiator.clone(),
                loser: dispute.initiator.clone(), // no respondent; loser is a placeholder
                reason: String::from_str(
                    &env,
                    "Respondent failed to respond within the resolution window",
                ),
                resolved_at: env.ledger().timestamp(),
            };
            dispute.status = DisputeStatus::Resolved;
            storage::set_dispute(&env, dispute_id, &dispute);
            storage::set_ruling(&env, dispute_id, &ruling);
            env.events()
                .publish(("resolve",), (dispute_id, ruling.winner.clone()));
            return Ok(ruling);
        }

        // Must be in Responded state to proceed with proof evaluation.
        if dispute.status != DisputeStatus::Responded {
            return Err(ContractError::NotResponded);
        }

        let respondent = dispute
            .respondent
            .clone()
            .ok_or(ContractError::NotResponded)?;

        // Evaluate by sequence number: lower sequence = originator = wins.
        let respondent_proof = match &dispute.respondent_proof {
            OptionalRelayChainProof::Some(p) => p.clone(),
            OptionalRelayChainProof::None => return Err(ContractError::NotResponded),
        };

        let (winner, loser, reason) =
            if dispute.initiator_proof.sequence <= respondent_proof.sequence {
                (
                    dispute.initiator.clone(),
                    respondent.clone(),
                    String::from_str(
                        &env,
                        "Initiator proof has lower or equal sequence; initiator wins",
                    ),
                )
            } else {
                (
                    respondent.clone(),
                    dispute.initiator.clone(),
                    String::from_str(&env, "Respondent proof has lower sequence; respondent wins"),
                )
            };

        let ruling = Ruling {
            dispute_id,
            winner: winner.clone(),
            loser,
            reason,
            resolved_at: env.ledger().timestamp(),
        };

        dispute.status = DisputeStatus::Resolved;
        storage::set_dispute(&env, dispute_id, &dispute);
        storage::set_ruling(&env, dispute_id, &ruling);

        env.events()
            .publish(("resolve",), (dispute_id, ruling.winner.clone()));

        Ok(ruling)
    }
}
