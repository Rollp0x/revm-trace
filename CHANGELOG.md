# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


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

