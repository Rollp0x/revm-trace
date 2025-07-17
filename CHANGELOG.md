# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [4.0.1] - 2025-7-17

### Added
- **NFT Support**: `TokenTransfer` now supports NFT transfers, including both ERC721 and ERC1155 tokens.
    - The `token_type` field distinguishes between ERC20, ERC721, ERC1155, and native tokens.
    - The `id` field records the tokenId for ERC721 and ERC1155 transfers.
- All transfer parsing and tracing APIs now handle ERC20, ERC721, ERC1155, and native asset transfers in a unified way.

### Changed
- `TokenTransfer` struct updated:
    - Added `token_type: TokenType` and `id: Option<U256>` fields.
    - Documentation and examples updated to reflect NFT support.


## [4.0.0] - 2025-06-24

### 🚀 Major Breaking Changes & Features

- **Unified Multi-Backend Support**: EVM simulation now supports both AlloyDB (default) and foundry-fork-db (via `foundry-fork` feature) with ergonomic, feature-conditional API.
- **EvmBuilder API**: All EVM construction is now via `EvmBuilder`, with `create_evm` and `create_evm_with_tracer` as quick entry points. Custom block height via builder or `set_db_block`.
- **Block Context Management**: All utility functions now manage block context at EVM creation; `block_env` removed from function signatures.
- **Multi-Threading & Concurrency**: Examples and API support robust multi-threaded and concurrent EVM simulation, including shared backend scenarios.
- **Trait-Based Reset & Caching**: New `ResetBlock` and `ResetDB` traits for block reset and cache management.
- **Improved Documentation**: README and doc comments rewritten for clarity, with new usage patterns, feature table, and a complete ERC20 transfer example.
- **Feature-Conditional Examples**: All examples modularized for backend selection via features.
- **Version Bump**: All version references updated to 4.0.0.

#### ⚠️ Breaking Changes
- All EVM construction and backend selection is now via `EvmBuilder` and feature flags.
- Utility functions no longer accept `block_env`; block context is set at EVM creation.
- API and example usage patterns have changed—see README for migration guidance.


## [3.1.1] - 2025-06-22

### 🎯 TxInspector Enhanced Support

#### New Features
- **Added specialized `get_inspector()` method for `TraceEvm<DB, TxInspector>`**
  - Provides direct access to TxInspector-specific methods without type erasure
  - Enables access to all TxInspector methods: `get_transfers()`, `get_traces()`, `get_logs()`, `get_error_trace_address()`, `find_error_trace()`
  - Solves the generic type access problem for advanced users who need full TxInspector functionality

#### Documentation Improvements
- **Comprehensive documentation overhaul**
  - Added clear explanation of three usage modes: Simple Execution, Manual Inspector Control, Automatic Batch Processing
  - Emphasized critical REVM API change: `evm.transact(tx)` does NOT execute Inspector
  - Added explicit requirement for `evm.inspect_replay_commit()` to activate Inspector
  - Included detailed mode selection guide and performance considerations
- **Enhanced code examples**
  - Added complete examples for all three usage modes
  - Fixed all import statements and API usage
  - Included error tracing examples and best practices

#### Key Insights
- **TxInspector is the core value proposition** - specialized support added
- **Modern REVM requires explicit Inspector activation** - clearly documented
- **Three distinct usage patterns** - each optimized for different scenarios

## [3.1.0] - 2025-06-22

### 🚀 Performance Improvements

#### Breaking Changes
- **Removed `Clone` constraint from `TraceInspector`**
  - Eliminates unnecessary inspector cloning constraints in batch processing
  - Enables users to use REVM's built-in `GasInspector` and `TracerEip3155` for custom wrapper development
  - Improves memory usage and performance
  - Simplifies trait bounds for better ergonomics

#### Internal Optimizations
- **Optimized `trace_internal` function**
  - Direct use of `set_tx()` + `inspect_replay_commit()` pattern
  - Eliminates intermediate inspector cloning
  - Better utilizes the `InspectCommitEvm` trait design

#### Documentation Updates
- Updated trait documentation to reflect removed `Clone` requirement
- Simplified examples without unnecessary derives
- Improved performance characteristics documentation

## [3.0.0] - 2025-06-22

### 🚀 Major Architecture Overhaul
This version represents a **complete rewrite** of the library with breaking changes to provide better performance, simplified APIs, and native multi-threading support.

### Added
- **🔥 Native Multi-Threading Support**
  - All EVM instances are now thread-safe by design
  - Optimized concurrent processing patterns
  - Built-in shared cache database for optimal performance
  - Support for high-throughput parallel transaction processing

- **⚡ Dual EVM Mode Architecture**
  - **Standard EVM Mode**: Ultra-fast execution with `create_evm()` (supports both HTTP and WebSocket)
  - **Tracing EVM Mode**: Comprehensive analysis with `create_evm_with_tracer()` (supports both HTTP and WebSocket)
  - Clear separation between performance-critical and analysis scenarios
  - `NoOpInspector` for zero-overhead execution in standard mode

- **🎯 Unified Type System**
  - Single `TraceEvm<CacheDB<SharedBackend>, INSP>` type for all scenarios
  - Eliminated `AlloyTraceEvm` and complex type variations
  - Consistent type aliases: `StandardEvm`, `TracingEvm<I>`
  - Clear generic constraints and trait bounds

- **🌐 Enhanced Protocol Support**
  - Native WebSocket support with dedicated builder functions
  - Automatic connection management and reconnection
  - HTTP and WebSocket providers with identical APIs
  - Flexible TLS backend support (native-tls by default, rustls optional)

- **📊 Advanced Inspector System**
  - Redesigned `TxInspector` with comprehensive trace collection
  - `TraceOutput` and `Reset` traits for standardized inspector behavior
  - Full support for custom REVM inspectors
  - Asset transfer tracking with detailed transaction analysis
  - Call hierarchy tracing with precise error location

- **🌐 Web Framework Integration**
  - **Actix-Web Integration Example**: Complete RESTful API implementation
  - **Multi-threading Support**: Two approaches using `web::block` and `tokio::task::spawn_blocking`
  - **Production-Ready**: Error handling, health checks, and JSON API responses
  - **Flexible Configuration**: Optional tracing with detailed call traces and asset transfers
  - **Performance Optimized**: Per-request EVM instance creation for optimal resource usage

### Changed
- **💥 BREAKING: Complete API Redesign**
  - Removed `EvmBuilder` pattern in favor of simple creation functions
  - `create_evm_instance_with_tracer()` → `create_evm_with_tracer()`
  - Eliminated complex configuration builders
  - Simplified transaction processing with `execute_batch()` and `trace_transactions()`

- **💥 BREAKING: Rewritten Core Modules**
  - **`multicall_utils.rs`**: Complete rewrite with improved error handling and type safety
  - **`evm/builder.rs`**: New implementation with unified provider creation
  - **`traits.rs`**: Expanded trait system with `TraceOutput`, `Reset`, `TransactionTrace`
  - **`types.rs`**: Redesigned type system with clear provider abstractions
  - **`errors.rs`**: Enhanced error hierarchy with better context

  - **💥 BREAKING: Transaction Processing**
  - New `SimulationBatch` structure with improved state management
  - Stateful/stateless execution modes with automatic nonce management
  - `process_transaction_internal()` → internal API, users use `execute_batch()`/`trace_transactions()`
  - Enhanced batch processing with per-transaction error handling

### Improved
- **🚀 Performance Optimizations**
  - Zero-allocation paths for standard execution mode
  - Optimized RPC connection management
  - Reduced memory footprint through smart caching
  - Concurrent processing patterns with minimal overhead

- **📚 Documentation & Examples**
  - Comprehensive API documentation with usage examples
  - Clear distinction between standard and tracing modes
  - Updated all examples to reflect new APIs
  - **Actix-Web Integration Guide**: Complete web API example with curl test commands
  - Performance guidance and best practices
  - Multi-threading usage patterns
  - Production deployment recommendations

- **🧪 Testing Infrastructure**
  - Expanded test coverage for all new features
  - Integration tests for both EVM modes
  - Concurrency testing with multi-threading scenarios
  - Documentation tests for all public APIs

### Fixed
- **Thread Safety Issues**
  - Eliminated race conditions in shared state access
  - Proper synchronization for concurrent EVM instances
  - Safe multi-threading patterns throughout the codebase

- **Type System Inconsistencies**
  - Resolved generic parameter conflicts
  - Fixed trait bound ambiguities
  - Consistent type constraints across all modules

- **Memory Management**
  - Fixed memory leaks in inspector implementations
  - Proper cleanup of RPC connections
  - Optimized cache invalidation strategies

### Removed
- **💥 BREAKING: Deprecated APIs**
  - `AlloyTraceEvm` type (replaced by unified `TraceEvm`)
  - `EvmBuilder` pattern (replaced by creation functions)
  - `create_evm_instance_with_tracer()` (replaced by `create_evm_with_tracer()`)
  - Complex configuration structs (simplified to function parameters)

### Migration Guide
```rust
// v2.x (deprecated)
let evm = EvmBuilder::new()
    .with_rpc_url("...")
    .with_tracer(inspector)
    .build().await?;

// v3.0 (new)
let evm = create_evm_with_tracer("...", inspector).await?;
```

### Dependencies
- Updated to latest REVM 24.0.1 for improved performance and stability
- Alloy 1.0.3 with enhanced provider capabilities
- Foundry-fork-db 0.15.1 for optimized database operations

### Technical Details
- **Type System Unification**
  - Single `TraceEvm<DB, INSP>` type replaces multiple EVM variants
  - Clear type aliases: `StandardEvm = TraceEvm<CacheDB<SharedBackend>, NoOpInspector>`
  - Consistent generic constraints across all modules
  
- **Inspector Architecture**
  - `NoOpInspector` implementation with `Reset` and `TraceOutput` traits
  - Custom `TxInspector` with comprehensive asset transfer tracking
  - Full call hierarchy analysis with error context

- **Provider System**
  - Layered filler pattern with automatic transaction field population
  - HTTP/WebSocket provider unification with secure TLS
  - Connection pooling and automatic reconnection handling

- **Batch Processing**
  - Dynamic nonce management for stateful execution
  - Per-transaction error isolation in batch operations
  - Memory-efficient processing of large transaction sets

- **Multicall System**
  - Universal deployment pattern for any EVM chain
  - Type-safe call construction and result parsing
  - Support for 100+ simultaneous contract calls

### Performance Improvements
- **Zero-Copy Processing**: Eliminated unnecessary allocations in hot paths
- **Concurrent Cache**: Shared database cache across multiple EVM instances
- **Smart Batching**: Automatic batch size optimization based on complexity
- **Memory Pooling**: Reusable buffers for frequent operations

### Breaking Changes Summary
| v2.x API | v3.0 API | Notes |
|----------|----------|-------|
| `EvmBuilder::new()` | `create_evm()` | Simplified creation |
| `AlloyTraceEvm` | `StandardEvm` | Unified type system |
| `create_evm_instance_with_tracer()` | `create_evm_with_tracer()` | Consistent naming |
| `process_transaction_internal()` | `execute_batch()`/`trace_transactions()` | User-facing APIs |

## [2.0.6] - 2025-06-15 [SUPERSEDED by 3.0.0]
### Added
- **New Multicall utilities module** (`multicall_utils.rs`) - **[REWRITTEN in 3.0.0]**
  - Universal Multicall solution that works on any EVM-compatible chain
  - Dynamic deployment of Multicall contract in simulation environment
  - Batch execution of multiple contract calls with individual error handling
  - Convenience functions for ERC20 balance and token info batch queries
  - Zero-dependency solution (no need for pre-deployed Multicall contracts)
  - Lightweight implementation without complex inspector requirements
- **Enhanced batch processing capabilities** - **[REDESIGNED in 3.0.0]**
  - Support for 100+ contract calls in a single batch operation
  - Optimized for cross-chain compatibility across all EVM networks
  - Efficient gas-free simulation environment for batch queries

### Examples
- Added `multicall_example.rs` demonstrating batch ERC20 balance queries - **[UPDATED in 3.0.0]**
- Added `test_contract_address.rs` showing contract deployment address extraction - **[UPDATED in 3.0.0]**

### Migration Note
⚠️  **This version is superseded by 3.0.0 which includes a complete rewrite of `multicall_utils` with improved type safety, better error handling, and native multi-threading support.**

## [2.0.5] - 2025-3-31
### Added
- Added `rustls-tls` feature to replace native-tls (OpenSSL) for improved cross-platform compatibility
- Simplified Docker builds and deployments by removing OpenSSL dependency
- Enhanced portability for environments where OpenSSL is unavailable or undesired

## [2.0.4] - 2025-3-2
### Fixed
- Added functionality to query the name and issuance of ERC20 tokens
- Added functionality to query the native token balance of an address

## [2.0.3] - 2025-2-13
### Fixed
- Fixed code execution issues in examples
- Added option to keep state after latest transaction in batch simulation for easier querying

## [2.0.2] - 2024-12-25

### Fixed
- Fixed delegate call stack tracing by always popping the address stack in `call_end`
- Fixed incorrect `from` address in delegate calls


## [2.0.1] - 2024-12-17

### Fixed
- Fixed a bug where the database was not being reset while starting a new simulation


## [2.0.0] - 2024-12-09 [SUPERSEDED by 3.0.0]

### Added - **[EXTENSIVELY REWRITTEN in 3.0.0]**
- Comprehensive trait system for extensible EVM functionality - **[REDESIGNED in 3.0.0]**
  - Full support for REVM's `Inspector` trait
  - Custom `TxInspector` for transaction analysis - **[COMPLETELY REWRITTEN in 3.0.0]**
  - `TraceOutput` for flexible result formatting - **[ENHANCED in 3.0.0]**
  - `TransactionProcessor` for standardized transaction handling - **[REDESIGNED in 3.0.0]**
  - `Reset` for state management - **[IMPROVED in 3.0.0]**
- Enhanced inspector implementation - **[MAJOR OVERHAUL in 3.0.0]**
  - Built on REVM's inspector system
  - Modular transaction analysis with `TxInspector`
  - Configurable trace collection
  - Improved state tracking
  - Easy integration with custom REVM inspectors
- WebSocket provider support with dedicated builder - **[SIMPLIFIED in 3.0.0]**
- Batch transaction processing with state management - **[REWRITTEN in 3.0.0]**
  - Stateful/stateless execution modes
  - Automatic state reset functionality
  - Concurrent execution support

### Changed - **[BREAKING CHANGES in 3.0.0]**
- Complete architecture redesign - **[FURTHER REDESIGNED in 3.0.0]**
  - New builder pattern for EVM creation - **[REPLACED with function-based API in 3.0.0]**
  - Improved error handling hierarchy - **[ENHANCED in 3.0.0]**
  - Better separation of concerns
  - More flexible configuration options - **[SIMPLIFIED in 3.0.0]**
- Enhanced transaction processing - **[MAJOR REWRITE in 3.0.0]**
  - Standardized execution flow
  - Improved error propagation
  - Better state management
- Modular inspector system - **[REDESIGNED in 3.0.0]**
  - Customizable trace collection
  - Flexible output formatting
  - State management utilities

### Migration Note
⚠️  **This version is superseded by 3.0.0 which includes breaking changes to the core architecture, API design, and introduces native multi-threading support. See 3.0.0 migration guide above.**

### Improved
- Documentation and examples
  - Comprehensive API documentation
  - Clear usage examples
  - Detailed error descriptions
- Error handling
  - New error type hierarchy
  - Better error context
  - Improved error messages
- Testing infrastructure
  - More comprehensive test cases
  - Better test utilities
  - Improved test coverage

### Features
- `ws` - WebSocket provider support
- `http` - HTTP provider support (default)
- `async` - Asynchronous execution support

## [1.0.0] - 2024-11-29

### Added
- Transaction log collection and analysis
- Simplified EVM instance creation with `create_evm_instance_with_tracer`
- Comprehensive documentation and examples
- Support for all major EVM-compatible chains

### Changed
- Simplified interface for transaction tracing
- Improved error handling and reporting
- Enhanced token transfer tracking
- Updated core data structures to include logs
- More detailed transaction execution traces

### Removed
- Complex configuration options in favor of simpler interfaces
- Unnecessary formatting functions


## [0.1.2] - 2024-11-26

### Added
- Added serde serialization support for all core types
  - Implemented `Serialize` and `Deserialize` for trace result types
  - Added snake_case serialization for enum variants
  - Improved third-party integration capabilities

## [0.1.1] - 2024-11-25

### Fixed
- Fixed a bug where transaction validation errors were not properly reported in the `TraceResult`
- Improved error handling to properly capture and display pre-execution validation failures


## [0.1.0] - 2024-11-24

### Added
- Initial implementation of EVM transaction simulation
- Asset transfer tracking functionality
  - Native token transfers (ETH, BNB, MATIC, etc.)
  - ERC20 token transfers
  - Internal transfer tracking
- Automatic token information collection
  - Token symbols
  - Token decimals
- Proxy contract support
  - EIP-1967 proxies
  - EIP-1967 beacon proxies
  - OpenZeppelin transparent proxies
  - EIP-1822 (UUPS) proxies
- Transaction tracing features
  - Detailed call traces
  - Error handling and revert messages
  - Asset transfer chronological ordering
- Multi-chain support
  - Compatible with all EVM-based chains
  - Configurable RPC endpoints
  - Historical state access

### Documentation
- Basic usage examples
- API documentation
- Integration test cases

