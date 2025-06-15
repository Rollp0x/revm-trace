# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.6] - 2025-06-15
### Added
- **New Multicall utilities module** (`multicall_utils.rs`)
  - Universal Multicall solution that works on any EVM-compatible chain
  - Dynamic deployment of Multicall contract in simulation environment
  - Batch execution of multiple contract calls with individual error handling
  - Convenience functions for ERC20 balance and token info batch queries
  - Zero-dependency solution (no need for pre-deployed Multicall contracts)
  - Lightweight implementation without complex inspector requirements
- **Enhanced batch processing capabilities**
  - Support for 100+ contract calls in a single batch operation
  - Optimized for cross-chain compatibility across all EVM networks
  - Efficient gas-free simulation environment for batch queries

### Examples
- Added `multicall_example.rs` demonstrating batch ERC20 balance queries
- Added `test_contract_address.rs` showing contract deployment address extraction

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


## [2.0.0] - 2024-12-09

### Added
- Comprehensive trait system for extensible EVM functionality
  - Full support for REVM's `Inspector` trait
  - Custom `TxInspector` for transaction analysis
  - `TraceOutput` for flexible result formatting
  - `TransactionProcessor` for standardized transaction handling
  - `Reset` for state management
- Enhanced inspector implementation
  - Built on REVM's inspector system
  - Modular transaction analysis with `TxInspector`
  - Configurable trace collection
  - Improved state tracking
  - Easy integration with custom REVM inspectors
- WebSocket provider support with dedicated builder
- Batch transaction processing with state management
  - Stateful/stateless execution modes
  - Automatic state reset functionality
  - Concurrent execution support

### Changed
- Complete architecture redesign
  - New builder pattern for EVM creation
  - Improved error handling hierarchy
  - Better separation of concerns
  - More flexible configuration options
- Enhanced transaction processing
  - Standardized execution flow
  - Improved error propagation
  - Better state management
- Modular inspector system
  - Customizable trace collection
  - Flexible output formatting
  - State management utilities

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

