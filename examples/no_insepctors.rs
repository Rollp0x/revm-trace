//! Basic Contract Deployment and Interaction Example
//! 
//! This example demonstrates:
//! - Basic EVM usage without an Inspector
//! - Contract deployment and method calling
//! - State management between transactions
//! - ABI encoding/decoding
//! 
//! Note: This is a simpler version compared to the traced example,
//! focusing on core functionality without execution tracing.

use revm_trace::{
    TransactionProcessor,
    traits::Database,
    types::TxKind,
    create_evm, SimulationBatch, SimulationTx,
};
use anyhow::Result;

use alloy::{
    primitives::{address, hex, Address, U256}, 
    sol, sol_types::SolCall
};
mod common;
use common::get_block_env;

// Define a simple contract that stores its deployer's address
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

// Contract bytecode generated from the above Solidity code
const BYTECODE:&str = "0x6080604052348015600f57600080fd5b50600080546001600160a01b031916331790556094806100306000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c8063893d20e814602d575b600080fd5b6033604f565b604080516001600160a01b039092168252519081900360200190f35b6000546001600160a01b03169056fea26469706673582212207e07a4e6666a33a6ee2fea8782ac8bcd42996a5130bb22b4353dbb5ea87bd4ee64736f6c63430007060033";
const ETH_RPC_URL: &str = "https://rpc.ankr.com/eth";
const SENDER: Address = address!("3ee18B2214AFF97000D974cf647E7C347E8fa585");

#[tokio::main]
async fn main() -> Result<()> {
    // Create basic EVM instance without inspector
    let mut evm = create_evm(ETH_RPC_URL).await.unwrap();

    // Calculate the contract address that will be created
    let current_account = evm.db_mut().basic(SENDER).unwrap().unwrap();
    let nonce = current_account.nonce;
    let owner_demo_address = SENDER.create(nonce);

    let block_env = get_block_env(ETH_RPC_URL, None).await.unwrap();

    // Transaction 1: Deploy the contract
    let data = hex::decode(BYTECODE).unwrap();
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

    let result = results[1].0.output().unwrap();
    let owner = Address::from_slice(&result[12..32]);
    println!("Owner: {:?}", owner);
    assert_eq!(owner, SENDER, "Owner should be the deployer");

    Ok(())
}