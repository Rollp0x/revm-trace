//! EVM simulation environment setup and configuration
//!
//! This module provides utilities for creating and configuring REVM instances
//! with various inspectors and development-friendly settings. It simplifies the
//! process of setting up an EVM environment for transaction simulation and tracing.
//!
//! # Key Features
//! - Configurable block number for historical state access
//! - Support for custom transaction inspectors
//! - Development-friendly default settings
//! - Flexible database configuration with AlloyDB
//!
//! # Historical State Access
//! When accessing historical blockchain state, the capabilities depend on the node type:
//!
//! - **Archive Nodes**: Can access any historical block state
//! - **Full Nodes**: Limited to recent blocks (typically ~128 blocks)
//!
//! The actual accessible block range varies by provider and node configuration.
//! Consider using archive nodes for deep historical analysis.
//!
//! # Example Usage
//! ```no_run
//! use revm_trace::evm::{create_evm_instance, create_evm_instance_with_inspector};
//! use revm_trace::TransactionTracer;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create EVM instance with default inspector
//! let evm = create_evm_instance(
//!     "https://eth-mainnet.example.com",
//!     None  // Use latest block
//! )?;
//!
//! // Create EVM instance with custom inspector
//! let evm = create_evm_instance_with_inspector(
//!     "https://eth-mainnet.example.com",
//!     TransactionTracer::default(),
//!     Some(17_000_000)  // Use specific block
//! )?;
//! # Ok(())
//! # }
//! ```

use alloy::{
    eips::{BlockId, BlockNumberOrTag},
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
/// Creates and configures an EVM instance with development-friendly settings and
/// the default NoOpInspector for basic transaction simulation.
///
/// # Arguments
/// * `rpc_url` - The URL of the Ethereum RPC endpoint (e.g., Infura, Alchemy, or local node)
/// * `block_number` - Optional block number for the simulation. If None, uses the latest block
///
/// # Returns
/// * `Ok(DefaultEvm)` - A configured EVM instance with NoOpInspector
/// * `Err(Error)` - If the RPC URL is invalid or connection fails
///
/// # Example
/// ```no_run
/// use revm_trace::evm::create_evm_instance;
///
/// # async fn example() -> anyhow::Result<()> {
/// let evm = create_evm_instance(
///     "https://eth-mainnet.alchemyapi.io/v2/your-api-key",
///     Some(17_000_000)  // Specific block number
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn create_evm_instance(rpc_url: &str, block_number: Option<u64>) -> Result<DefaultEvm> {
    let inspector = NoOpInspector;
    create_evm_instance_with_inspector(rpc_url, inspector, block_number)
}

/// Creates an EVM instance with a custom inspector
///
/// # Arguments
/// * `rpc_url` - RPC endpoint URL
/// * `inspector` - Custom inspector implementation
/// * `block_number` - Optional block number for historical state access
///   - None: Uses latest block
///   - Some(number): Uses specified block
///
/// # Node State Access Limitations
/// Historical state access capabilities depend on the node type and configuration:
///
/// - **Archive Nodes**
///   - Can access state from any historical block
///   - Typically provided by premium services (Alchemy, QuickNode, etc.)
///   - Required for deep historical analysis
///
/// - **Full Nodes**
///   - Limited historical state access
///   - Actual block depth varies by node configuration
///   - Default ranges vary by provider:
///     - Infura: ~128 blocks
///     - Alchemy: ~128 blocks (without archive access)
///     - Custom nodes: Depends on configuration
///
/// # Example
/// ```no_run
/// use revm_trace::{create_evm_instance_with_inspector, TransactionTracer};
///
/// # fn main() -> anyhow::Result<()> {
/// // Use latest block
/// let evm = create_evm_instance_with_inspector(
///     "https://eth-mainnet.g.alchemy.com/v2/YOUR-API-KEY",
///     TransactionTracer::default(),
///     None
/// )?;
///
/// // Use specific block (ensure your node supports historical access)
/// let evm = create_evm_instance_with_inspector(
///     "https://eth-mainnet.g.alchemy.com/v2/YOUR-API-KEY",
///     TransactionTracer::default(),
///     Some(17_000_000)
/// )?;
/// # Ok(())
/// # }
/// ```
///
/// # Note
/// When accessing historical state:
/// - Verify your node type (archive vs full)
/// - Check provider documentation for specific limitations
/// - Consider using archive nodes for deep historical analysis
/// - Failed state access will result in runtime errors
pub fn create_evm_instance_with_inspector<I>(
    rpc_url: &str,
    inspector: I,
    block_number: Option<u64>,
) -> Result<InspectorEvm<I>>
where
    I: Inspector<EvmDb> + GetInspector<EvmDb>,
{
    let provider = ProviderBuilder::new().on_http(rpc_url.parse()?);

    let block_id = match block_number {
        Some(number) => BlockId::Number(BlockNumberOrTag::Number(number)),
        None => BlockId::latest(),
    };

    let alloy_db = AlloyDB::new(provider, block_id).ok_or_else(|| {
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

    Ok(evm)
}
