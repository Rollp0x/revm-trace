//! Error handling utilities for Ethereum transactions
//!
//! This module provides utilities for parsing and handling various types of errors:
//! - Custom Solidity errors (Error(string))
//! - Solidity panic codes (Panic(uint256))
//! - Custom error selectors
//!
//! Common error scenarios that this module handles:
//! - Revert with string message
//! - Assertion failures
//! - Arithmetic operations
//! - Array bounds checks

use alloy::dyn_abi::{DynSolType, DynSolValue};

/// Parse custom error output from a failed transaction
///
/// Handles two main types of errors:
/// 1. Error(string) - Standard revert with message (selector: 0x08c379a0)
/// 2. Panic(uint256) - Solidity panic with error code (selector: 0x4e487b71)
///
/// # Arguments
/// * `output` - Raw error output bytes from the failed transaction
///
/// # Returns
/// * `Some(String)` - Decoded error message or panic reason
/// * `None` - If the error format is not recognized or cannot be decoded
///
/// # Example
/// ```no_run
/// # use your_crate::error_utils::parse_custom_error;
/// let error_output = hex::decode("08c379a000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000014496e73756666696369656e742062616c616e636500000000000000000000000000").unwrap();
/// let error_message = parse_custom_error(&error_output);
/// assert_eq!(error_message, Some("Insufficient balance".to_string()));
/// ```
pub fn parse_custom_error(output: &[u8]) -> Option<String> {
    if output.len() < 4 {
        return None;
    }

    let selector = &output[0..4];
    match selector {
        // Error(string) - 0x08c379a0
        [0x08, 0xc3, 0x79, 0xa0] => {
            if let Ok(DynSolValue::String(reason)) = DynSolType::String.abi_decode(&output[4..]) {
                Some(reason)
            } else {
                None
            }
        },
        // Panic(uint256) - 0x4e487b71
        [0x4e, 0x48, 0x7b, 0x71] => {
            if let Ok(DynSolValue::Uint(code, _)) = DynSolType::Uint(256).abi_decode(&output[4..]) {
                return Some(match code.to::<u64>() {
                    0x01 => "Panic: Assertion failed".to_string(),
                    0x11 => "Panic: Arithmetic overflow".to_string(),
                    0x12 => "Panic: Division by zero".to_string(),
                    0x21 => "Panic: Invalid array access".to_string(),
                    0x22 => "Panic: Array access out of bounds".to_string(),
                    0x31 => "Panic: Invalid enum value".to_string(),
                    0x32 => "Panic: Invalid storage access".to_string(),
                    0x41 => "Panic: Zero initialization".to_string(),
                    0x51 => "Panic: Invalid calldata access".to_string(),
                    code => format!("Panic: Unknown error code (0x{:x})", code),
                });
            }
            None
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::hex::decode;

    #[test]
    fn test_parse_error_string() {
        // "Insufficient balance" encoded as Error(string)
        let error_bytes = decode(
            "08c379a0\
             0000000000000000000000000000000000000000000000000000000000000020\
             0000000000000000000000000000000000000000000000000000000000000014\
             496e73756666696369656e742062616c616e636500000000000000000000000000" 
        ).unwrap();

        let result = parse_custom_error(&error_bytes);
        assert_eq!(result, Some("Insufficient balance".to_string()));

        // Test invalid error string format
        let invalid_bytes = decode("08c379a0").unwrap();
        assert_eq!(parse_custom_error(&invalid_bytes), None);
    }

    #[test]
    fn test_parse_panic() {
        // Test various panic codes
        let panic_codes = [
            (0x01, "Panic: Assertion failed"),
            (0x11, "Panic: Arithmetic overflow"),
            (0x12, "Panic: Division by zero"),
            (0x21, "Panic: Invalid array access"),
            (0x22, "Panic: Array access out of bounds"),
            (0x31, "Panic: Invalid enum value"),
            (0x32, "Panic: Invalid storage access"),
            (0x41, "Panic: Zero initialization"),
            (0x51, "Panic: Invalid calldata access"),
            (0xFF, "Panic: Unknown error code (0xff)"),
        ];

        for (code, expected_message) in panic_codes {
            // Encode Panic(uint256)
            let panic_bytes = [
                // Selector for Panic(uint256)
                0x4e, 0x48, 0x7b, 0x71,
                // Encoded uint256 value (32 bytes, left-padded)
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, code as u8,
            ];

            let result = parse_custom_error(&panic_bytes);
            assert_eq!(result, Some(expected_message.to_string()));
        }
    }

    #[test]
    fn test_invalid_inputs() {
        // Test empty input
        assert_eq!(parse_custom_error(&[]), None);

        // Test input shorter than selector
        assert_eq!(parse_custom_error(&[0x08, 0xc3, 0x79]), None);

        // Test unknown selector
        assert_eq!(parse_custom_error(&[0x00, 0x00, 0x00, 0x00]), None);

        // Test invalid panic code encoding
        let invalid_panic = [
            0x4e, 0x48, 0x7b, 0x71,  // Panic selector
            0x00  // Invalid uint256 encoding
        ];
        assert_eq!(parse_custom_error(&invalid_panic), None);
    }
}