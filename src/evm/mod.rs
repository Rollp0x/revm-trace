//! EVM simulation environment setup and configuration
//!
//! This module provides utilities for creating and configuring REVM instances
//! with either NoOpInspector or TransactionTracer. It simplifies the process of 
//! setting up an EVM environment for transaction simulation and tracing.
//!
//! # Key Features
//! - Chain ID configuration for different networks
//! - Development-friendly default settings
//! - Two inspector options:
//!   - NoOpInspector for basic simulation
//!   - TransactionTracer for detailed execution tracing
//! - Flexible database configuration with AlloyDB
//!
//! # Example Usage
//! ```no_run
//! use revm_trace::evm::{create_evm_instance, create_evm_instance_with_tracer};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create EVM instance with NoOpInspector for Ethereum mainnet
//! let evm = create_evm_instance(
//!     "https://eth-mainnet.example.com",
//!     Some(1)  // Ethereum mainnet chain ID
//! )?;
//!
//! // Create EVM instance with TransactionTracer for Goerli testnet
//! let evm = create_evm_instance_with_tracer(
//!     "https://eth-goerli.example.com",
//!     Some(5)  // Goerli testnet chain ID
//! )?;
//! # Ok(())
//! # }
//! ```

use alloy::{
    eips::BlockId,
    network::Ethereum,
    providers::{ProviderBuilder, RootProvider},
    transports::http::{Client, Http},
};

use anyhow::{anyhow, Result};
use revm::{
    db::{AlloyDB, WrapDatabaseRef},
    inspector_handle_register,
    inspectors::NoOpInspector,
    Evm, GetInspector, Inspector,
};

use crate::{Reset, TransactionTracer};

/// Type alias for the database used in EVM instances.
///
/// Uses WrapDatabaseRef to wrap AlloyDB with HTTP transport for Ethereum network access.
/// This type provides a reference-counted wrapper around AlloyDB to allow sharing
/// across multiple components while maintaining proper lifetime management.
pub type EvmDb = WrapDatabaseRef<AlloyDB<Http<Client>, Ethereum, RootProvider<Http<Client>>>>;

/// Type alias for the default EVM instance using NoOpInspector.
///
/// This type is used when no custom inspector is needed for basic transaction simulation.
/// The NoOpInspector provides a minimal implementation that performs no additional tracking
/// or analysis during transaction execution.
pub type DefaultEvm = Evm<'static, NoOpInspector, EvmDb>;

/// Type alias for EVM instances with custom inspectors.
///
/// # Type Parameters
/// * `I` - The inspector type to be used with the EVM. Must implement both `Inspector`
///         and `GetInspector` traits for the `EvmDb` database type.
pub type InspectorEvm<I> = Evm<'static, I, EvmDb>;

/// Creates a new EVM instance with default configuration using NoOpInspector.
///
/// # Arguments
/// * `rpc_url` - The URL of the Ethereum RPC endpoint (e.g., Infura, Alchemy, or local node)
/// * `chain_id` - Optional chain ID for the EVM environment
///   - None: Uses default chain ID
///   - Some(id): Uses specified chain ID (e.g., 1 for Ethereum Mainnet)
///
/// # Returns
/// * `Ok(DefaultEvm)` - A configured EVM instance with NoOpInspector
/// * `Err(Error)` - If the RPC URL is invalid or connection fails
///
/// # Example
/// ```no_run
/// use revm_trace::evm::create_evm_instance;
///
/// # fn main() -> anyhow::Result<()> {
/// let evm = create_evm_instance(
///     "https://eth-mainnet.example.com",
///     Some(1)  // Ethereum mainnet
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn create_evm_instance(rpc_url: &str, chain_id: Option<u64>) -> Result<DefaultEvm> {
    create_evm_internal(rpc_url, NoOpInspector, chain_id)
}

/// Creates an EVM instance with TransactionTracer for detailed execution tracing.
///
/// # Arguments
/// * `rpc_url` - The URL of the Ethereum RPC endpoint
/// * `chain_id` - Optional chain ID for the EVM environment
///   - None: Uses default chain ID
///   - Some(id): Uses specified chain ID (e.g., 1 for Ethereum Mainnet)
///
/// # Returns
/// * `Ok(InspectorEvm<TransactionTracer>)` - A configured EVM instance with TransactionTracer
/// * `Err(Error)` - If the RPC URL is invalid or connection fails
///
/// # Features
/// - Tracks all EVM operations
/// - Records token transfers
/// - Captures contract interactions
/// - Provides detailed execution traces
///
/// # Example
/// ```no_run
/// use revm_trace::evm::create_evm_instance_with_tracer;
///
/// # fn main() -> anyhow::Result<()> {
/// let evm = create_evm_instance_with_tracer(
///     "https://eth-goerli.example.com",
///     Some(5)  // Goerli testnet
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn create_evm_instance_with_tracer(
    rpc_url: &str,
    chain_id: Option<u64>,
) -> Result<InspectorEvm<TransactionTracer>> {
    create_evm_internal(rpc_url, TransactionTracer::default(), chain_id)
}

/// Internal function to create EVM instance with common configuration.
/// 
/// This function handles the common setup logic for both inspector types:
/// - Creates and configures AlloyDB with the provided RPC URL
/// - Sets up development-friendly EVM settings
/// - Configures chain ID if provided
/// - Initializes the inspector
fn create_evm_internal<I>(
    rpc_url: &str,
    inspector: I,
    chain_id: Option<u64>,
) -> Result<InspectorEvm<I>>
where
    I: Inspector<EvmDb> + GetInspector<EvmDb> + Reset + 'static,
{
    let provider = ProviderBuilder::new().on_http(rpc_url.parse()?);
    let alloy_db = AlloyDB::new(provider, BlockId::latest()).ok_or_else(|| {
        anyhow!(
            "Failed to create AlloyDB. Possible reasons:\n\
             1. Invalid RPC URL: {}\n\
             2. No tokio runtime available\n\
             3. Current runtime is single-threaded\n\
             4. Requested block state not available\n\
             Solutions:\n\
             - Use #[tokio::test(flavor = \"multi_thread\")] for tests\n\
             - Use an archive node for historical state access\n\
             - Check if the block number is within node's state range",
            rpc_url
        )
    })?;

    let mut evm = Evm::builder()
        .with_ref_db(alloy_db)
        .with_external_context(inspector)
        .append_handler_register(inspector_handle_register)
        .build();

    // Configure development-friendly settings
    let cfg = evm.cfg_mut();
    cfg.disable_eip3607 = true;
    cfg.disable_block_gas_limit = true;
    cfg.limit_contract_code_size = None;
    cfg.disable_base_fee = true;

    if let Some(chain_id) = chain_id {
        cfg.chain_id = chain_id;
        evm.tx_mut().chain_id = Some(chain_id);
    }
    
    Ok(evm)
}
