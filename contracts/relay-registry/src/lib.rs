//! # Relay Registry Contract — `lib.rs`
//!
//! This is the main entry point for the Relay Registry Soroban smart contract.
//! It exposes the public contract interface and wires together the types, storage,
//! and error modules.
//!
//! ## Responsibilities
//! - Relay node registration on-chain (`register`)
//! - Token staking and unstaking with lock period enforcement (`stake`, `unstake`)
//! - Stake slashing for misbehaving relay nodes (`slash`)
//! - Node lookup and active-status verification (`get_node`, `is_active`)
//!
//! ## Functions
//! - `register(env, node_address, metadata)` — Register a new relay node with metadata
//! - `stake(env, amount)` — Deposit stake tokens into the registry
//! - `unstake(env, amount)` — Initiate stake withdrawal, subject to lock period
//! - `slash(env, node_address, reason)` — Slash a misbehaving relay node's stake
//! - `get_node(env, address)` — Fetch relay node details and metadata
//! - `is_active(env, address)` — Check if a relay node is currently in active status
//!
//! ## See also
//! - `types.rs` — Data structures (RelayNode, NodeMetadata, NodeStatus)
//! - `storage.rs` — Persistent storage helpers
//! - `errors.rs` — Contract error codes
//!
//! implementation tracked in GitHub issue

#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, String};

pub mod errors;
pub mod storage;
pub mod types;

use crate::errors::ContractError;
use crate::types::{NodeMetadata, NodeStatus, RelayNode};

#[contract]
pub struct RelayRegistryContract;

#[contractimpl]
impl RelayRegistryContract {
    /// Register a new relay node with the given address and metadata.
    ///
    /// # Parameters
    /// - `env`: Soroban environment for the current contract invocation.
    /// - `node_address`: Stellar account address of the relay node. Must authorize this call.
    /// - `metadata`: Metadata describing the relay node's region, capacity, and uptime commitment.
    ///
    /// # Errors
    /// - `ContractError::AlreadyRegistered` if a node with this address already exists.
    /// - `ContractError::InvalidMetadata` if `metadata.uptime_commitment` is greater than 100.
    pub fn register(
        env: Env,
        node_address: Address,
        metadata: NodeMetadata,
    ) -> Result<(), ContractError> {
        node_address.require_auth();

        if storage::get_node(&env, &node_address).is_some() {
            return Err(ContractError::AlreadyRegistered);
        }

        if metadata.uptime_commitment > 100 {
            return Err(ContractError::InvalidMetadata);
        }

        let timestamp = env.ledger().timestamp();

        let node = RelayNode {
            address: node_address.clone(),
            stake: 0,
            status: NodeStatus::Inactive,
            metadata,
            registered_at: timestamp,
            last_active: timestamp,
        };

        storage::set_node(&env, &node_address, &node);
        storage::increment_node_count(&env);
        Ok(())
    }

    pub fn stake(
        env: Env,
        node_address: Address,
        amount: i128,
    ) -> Result<RelayNode, ContractError> {
        node_address.require_auth();
        if amount <= 0 {
            return Err(ContractError::InsufficientStake);
        }

        let mut node =
            storage::get_node(&env, &node_address).ok_or(ContractError::NotRegistered)?;
        if matches!(node.status, NodeStatus::Slashed) {
            return Err(ContractError::NodeSlashed);
        }

        node.stake = node
            .stake
            .checked_add(amount)
            .ok_or(ContractError::Overflow)?;

        if node.stake < storage::get_min_stake(&env) {
            return Err(ContractError::InsufficientStake);
        }

        node.status = NodeStatus::Active;
        node.last_active = env.ledger().timestamp();
        storage::set_node(&env, &node_address, &node);
        Ok(node)
    }

    pub fn unstake(
        env: Env,
        node_address: Address,
        amount: i128,
    ) -> Result<RelayNode, ContractError> {
        node_address.require_auth();
        if amount <= 0 {
            return Err(ContractError::InsufficientStake);
        }

        let mut node =
            storage::get_node(&env, &node_address).ok_or(ContractError::NotRegistered)?;
        if matches!(node.status, NodeStatus::Slashed) {
            return Err(ContractError::NodeSlashed);
        }
        if !matches!(node.status, NodeStatus::Active) {
            return Err(ContractError::NodeNotActive);
        }

        let unlock_after = node
            .registered_at
            .checked_add(storage::get_stake_lock_period(&env) as u64)
            .ok_or(ContractError::Overflow)?;
        if env.ledger().timestamp() < unlock_after {
            return Err(ContractError::StakeLocked);
        }
        if amount > node.stake {
            return Err(ContractError::InsufficientStake);
        }

        node.stake = node
            .stake
            .checked_sub(amount)
            .ok_or(ContractError::Overflow)?;

        if node.stake < storage::get_min_stake(&env) {
            node.status = NodeStatus::Inactive;
        }
        node.last_active = env.ledger().timestamp();
        storage::set_node(&env, &node_address, &node);
        Ok(node)
    }

    pub fn slash(
        env: Env,
        node_address: Address,
        _reason: String,
    ) -> Result<RelayNode, ContractError> {
        let mut node =
            storage::get_node(&env, &node_address).ok_or(ContractError::NotRegistered)?;
        if matches!(node.status, NodeStatus::Slashed) {
            return Err(ContractError::NodeSlashed);
        }

        node.stake = 0;
        node.status = NodeStatus::Slashed;
        node.last_active = env.ledger().timestamp();
        storage::set_node(&env, &node_address, &node);
        Ok(node)
    }

    pub fn get_node(env: Env, address: Address) -> Result<RelayNode, ContractError> {
        storage::get_node(&env, &address).ok_or(ContractError::NotRegistered)
    }

    pub fn is_active(env: Env, address: Address) -> bool {
        matches!(
            storage::get_node(&env, &address).map(|n| n.status),
            Some(NodeStatus::Active)
        )
    }
}
