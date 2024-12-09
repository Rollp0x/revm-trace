//! Proxy contract analysis and implementation resolution
//!
//! This module provides utilities for:
//! - Detecting proxy contracts
//! - Resolving implementation addresses
//! - Supporting multiple proxy patterns:
//!   - EIP-1967 (Transparent Proxy)
//!   - EIP-1822 (UUPS Proxy)
//!   - OpenZeppelin Proxy
//!   - Beacon Proxy

use alloy::{
    network::Ethereum, primitives::{Address, U256}, providers::Provider, transports::Transport
};
use anyhow::Result;
use once_cell::sync::Lazy;
use revm::{db::{AlloyDB, CacheDB, Database, WrapDatabaseRef}, Inspector};
use std::str::FromStr;
use crate::{evm::TraceEvm, BlockEnv};
use crate::errors::{EvmError,RuntimeError};
/// Slot for EIP-1967 implementation address
///
/// Calculated as: keccak256("eip1967.proxy.implementation") - 1
const EIP_1967_LOGIC_SLOT: &str =
    "0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc";

/// Storage slot for EIP-1967 beacon address
///
/// Calculated as: keccak256("eip1967.proxy.beacon") - 1
const EIP_1967_BEACON_SLOT: &str =
    "0xa3f0ad74e5423aebfd80d3ef4346578335a9a72aeaee59ff6cb3582b35133d50";

/// Storage slot for OpenZeppelin implementation address
///
/// Calculated as: keccak256("eip1967.proxy.implementation") - 1
const OZ_IMPLEMENTATION_SLOT: &str =
    "0x7050c9e0f4ca769c69bd3a8ef740bc37934f8e2c036e5a723fd8ee048ed3f8c3";

/// Storage slot for EIP-1822 implementation address
///
/// Calculated as: keccak256("eip1822.proxy.implementation") - 1
const EIP_1822_LOGIC_SLOT: &str =
    "0xc5f16f0fcc639fa48a6947836d9850f504798523bf8c9a3a87d5876cf622bcf7";

/// Storage slots for different proxy patterns
static IMPLEMENTATION_SLOTS: Lazy<Vec<U256>> = Lazy::new(|| {
    vec![
        // EIP-1967 implementation slot
        U256::from_str(EIP_1967_LOGIC_SLOT).unwrap(),
        // EIP-1967 beacon slot
        U256::from_str(EIP_1967_BEACON_SLOT).unwrap(),
        // OpenZeppelin implementation slot
        U256::from_str(OZ_IMPLEMENTATION_SLOT).unwrap(),
        // EIP-1822 implementation slot
        U256::from_str(EIP_1822_LOGIC_SLOT).unwrap(),
    ]
});

/// Attempts to find the implementation address for a proxy contract
///
/// Checks multiple proxy patterns to find the implementation contract address.
/// Supports the following proxy patterns:
/// - EIP-1967 Transparent Proxy
/// - EIP-1967 Beacon Proxy
/// - OpenZeppelin Legacy Proxy
/// - EIP-1822 Universal Upgradeable Proxy (UUPS)
///
/// # Arguments
/// * `evm` - Configured EVM instance for state access
/// * `contract` - Address of the potential proxy contract
///
/// # Returns
/// * `Ok(Some(Address))` - Implementation address if found
/// * `Ok(None)` - If no implementation is found (might not be a proxy)
/// * `Err(_)` - If there's an error accessing contract state
///
/// # Example
/// ```no_run
/// use revm_trace::utils::proxy_utils::get_implement;
/// use alloy::primitives::address;
///
/// # async fn example() -> anyhow::Result<()> {
/// # let mut evm = todo!();
/// // USDT proxy contract
/// let proxy = address!("dac17f958d2ee523a2206206994597c13d831ec7");
///
/// if let Some(implementation) = get_implement(&mut evm, proxy).await? {
///     println!("Implementation found at: {}", implementation);
/// } else {
///     println!("No implementation found (not a proxy)");
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Implementation Details
/// The function:
/// 1. Checks each known implementation slot in order
/// 2. For non-zero values, attempts to convert to an address
/// 3. Verifies the address has deployed code
/// 4. Returns the first valid implementation found
///
/// # Common Proxy Patterns
/// - EIP-1967: Modern transparent proxy pattern
/// - EIP-1822: Universal Upgradeable Proxy Standard (UUPS)
/// - OpenZeppelin: Legacy proxy implementation
/// - Beacon: Proxy pattern for multiple contracts sharing same implementation
pub fn get_implement<T, P,I>(
    evm: &mut TraceEvm<'_, T, P,I>,
    proxy: Address,
    block_env: Option<BlockEnv>,
) -> Result<Option<Address>, EvmError>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: Inspector<WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>>,
{   
    if let Some(block_env) = block_env {
        evm.set_block_env(block_env);
    }
    // First verify if the contract exists
    if evm.db_mut().basic(proxy)
        .map_err(|e| RuntimeError::AccountAccess(format!("Get contract {} state failed: {}",proxy,e)))?
        .is_none()
    {
        return Ok(None);
    }
    // Check each possible implementation slot
    for &slot in IMPLEMENTATION_SLOTS.iter() {
        let value = evm.db_mut().storage(proxy, slot)
            .map_err(|e| RuntimeError::SlotAccess(format!("Get contract {} slot {} state failed: {}",proxy,slot,e)))?;
        if value != U256::ZERO {
            let impl_address = Address::from_slice(&value.to_be_bytes::<32>()[12..32]);

            // Only verify if the implementation account exists
            if let Some(impl_acc) = evm.db_mut().basic(impl_address)
                .map_err(|e| RuntimeError::AccountAccess(format!("Get implementation {} state failed: {}",impl_address,e)))? {
                // Check if account has code without loading it
                if !impl_acc.code_hash.is_zero() {
                    return Ok(Some(impl_address));
                }
            }
        }
    }

    Ok(None)
}
