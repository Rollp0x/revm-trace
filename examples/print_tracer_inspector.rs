//! Custom Print Tracer Example
//! 
//! This example demonstrates:
//! - Using revm's built-in CustomPrintTracer
//! - Detailed opcode-level execution tracing
//! - Human-readable output format
//! - Contract deployment and interaction with tracing
//! 
//! The CustomPrintTracer provides a more readable format compared to EIP-3155,
//! showing depth, PC, gas, opcode, and stack information in a clear format.

use revm_trace::{
    TransactionProcessor,
    traits::Database,
    types::TxKind,
    inspectors::CustomPrintTracer,
    create_evm_with_inspector, SimulationBatch, SimulationTx,
};
use anyhow::Result;
use alloy::{
    primitives::{address, hex, Address, U256}, 
    sol, sol_types::SolCall
};
mod common;
use common::get_block_env;

// Define contract with owner storage
sol! {
    contract OwnerDemo {
        address private _owner;
        constructor() {
            _owner = msg.sender;
        }

        function getOwner() public view returns (address) {
            return _owner;
        }
    }
}

const BYTECODE:&str = "0x6080604052348015600f57600080fd5b50600080546001600160a01b031916331790556094806100306000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c8063893d20e814602d575b600080fd5b6033604f565b604080516001600160a01b039092168252519081900360200190f35b6000546001600160a01b03169056fea26469706673582212207e07a4e6666a33a6ee2fea8782ac8bcd42996a5130bb22b4353dbb5ea87bd4ee64736f6c63430007060033";
const ETH_RPC_URL: &str = "https://rpc.ankr.com/eth";
const SENDER: Address = address!("3ee18B2214AFF97000D974cf647E7C347E8fa585");

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize EVM with CustomPrintTracer
    let inspector = CustomPrintTracer::default();
    let mut evm = create_evm_with_inspector(ETH_RPC_URL, inspector).await.unwrap();

    // Calculate the contract address that will be created
    let current_account = evm.db_mut().basic(SENDER).unwrap().unwrap();
    let nonce = current_account.nonce;
    let owner_demo_address = SENDER.create(nonce);
    let block_env = get_block_env(ETH_RPC_URL, None).await.unwrap();
    let data = hex::decode(BYTECODE).unwrap();

    // Transaction 1: Deploy the contract
    let tx0 = SimulationTx {    
        caller: SENDER,
        transact_to: TxKind::Create,
        value: U256::ZERO,
        data: data.clone().into(),
    };

    // Transaction 2: Call getOwner() on the deployed contract
    let tx1 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Call(owner_demo_address),
        value: U256::ZERO,
        data: OwnerDemo::getOwnerCall{}.abi_encode().into(),
    };

    // Process both transactions with state preservation
    let results = evm.process_transactions(SimulationBatch {
        block_env,
        is_stateful: true,
        transactions: vec![tx0,tx1],
    }).into_iter().map(|v| v.unwrap()).collect::<Vec<_>>();

    // Output the result of the second transaction
    let result = results[1].0.output().unwrap();
    let owner = Address::from_slice(&result[12..32]);
    println!("Owner: {:?}", owner);
    assert_eq!(owner, SENDER, "Owner should be the deployer");

    Ok(())
}

// Example trace output for a single opcode:
// depth:1,            // Call depth in the EVM
// PC:0,              // Program Counter
// gas:0xfff...(n),   // Remaining gas (with decimal value)
// OPCODE: "PUSH1"(96) // Opcode name and value
// refund:0x0(0)      // Gas refund
// Stack:[],          // Current stack contents
// Data size:0        // Size of memory data