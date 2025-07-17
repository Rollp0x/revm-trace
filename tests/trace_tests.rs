//! Integration tests for transaction tracing and asset tracking
//!
//! This test module verifies the transaction simulation and asset tracing functionality
//! across different scenarios:
//!
//! # Test Coverage
//! - Historical state access at different block heights
//! - Different caller types (EOA vs Contract addresses)
//! - Different inspector configurations
//! - Contract deployment and interaction
//! - Error handling and revert scenarios
//! - Multicall transaction batching
//! - State changes between transactions
//!
//! # Test Infrastructure
//! - Uses Ankr's public RPC endpoint
//! - Requires multi-threaded tokio runtime
//! - Tests both successful and failure cases
//!
//! # Note on Historical State Access
//! The tests include scenarios for accessing historical state, but success depends
//! on the RPC node's capabilities:
//! - Recent blocks: May succeed on regular nodes
//! - Old blocks: Requires archive node access

use revm::context::ContextTr;
use revm::database::Database;
use revm_trace::{
    create_evm_with_tracer, utils::error_utils::parse_custom_error, SimulationBatch, SimulationTx,
    TransactionTrace, TxInspector,
};

use alloy::{
    primitives::{address, hex, Address, TxKind, U256},
    sol,
    sol_types::SolCall,
};

// Test contract definitions using alloy-sol macro
sol! {
    contract OwnerDemo {
        address public owner;
        address public revert_address;

        constructor() {
            owner = msg.sender;
        }

        function setOwner(address _owner) public {
            require(msg.sender == owner, "Only the owner can set the owner");
            owner = _owner;
        }

        function setRevertDemo(address _revert_address) public {
            revert_address = _revert_address;
        }

        function revert_demo() public {
            RevertDemo(revert_address).revert_demo();
        }


        function revert_demo_multi() public {
            // catch first call
            try RevertDemo(revert_address).revert_demo() {
            } catch Error(string memory /*reason*/) {
                // catch revert error
            } catch (bytes memory /*lowLevelData*/) {
                // catch other errors
            }

            // second call will cause actual revert
            RevertDemo(revert_address).revert_demo();
        }
    }

    contract RevertDemo {
        function revert_demo() public {
            this.nested_revert();
        }

        function nested_revert() public {
            revert("Revert demo");
        }
    }
}

const ETH_RPC_URL: &str = "https://eth.llamarpc.com";
const SENDER: Address = address!("3ee18B2214AFF97000D974cf647E7C347E8fa585");
const CAFE_ADDRESS: Address = address!("cafe00000000000000000000000000000000face");
const DEAD_ADDRESS: Address = address!("deAD00000000000000000000000000000000dEAd");
const OWNER_DEMO_BYTECODE:&str = "0x608060405234801561001057600080fd5b50600080546001600160a01b031916331790556103ae806100326000396000f3fe608060405234801561001057600080fd5b50600436106100625760003560e01c806313af40351461006757806315bb76871461008f5780633d39ef1f146100b55780635e56f344146100bd5780638da5cb5b146100c5578063f106e187146100e9575b600080fd5b61008d6004803603602081101561007d57600080fd5b50356001600160a01b03166100f1565b005b61008d600480360360208110156100a557600080fd5b50356001600160a01b0316610172565b61008d610194565b61008d610244565b6100cd6102ae565b604080516001600160a01b039092168252519081900360200190f35b6100cd6102bd565b6000546001600160a01b03163314610150576040805162461bcd60e51b815260206004820181905260248201527f4f6e6c7920746865206f776e65722063616e2073657420746865206f776e6572604482015290519081900360640190fd5b600080546001600160a01b0319166001600160a01b0392909216919091179055565b600180546001600160a01b0319166001600160a01b0392909216919091179055565b600160009054906101000a90046001600160a01b03166001600160a01b0316635e56f3446040518163ffffffff1660e01b8152600401600060405180830381600087803b1580156101e457600080fd5b505af19250505080156101f5575060015b610244576102016102d2565b8061020c5750610212565b50610244565b3d80801561023c576040519150601f19603f3d011682016040523d82523d6000602084013e610241565b606091505b50505b600160009054906101000a90046001600160a01b03166001600160a01b0316635e56f3446040518163ffffffff1660e01b8152600401600060405180830381600087803b15801561029457600080fd5b505af11580156102a8573d6000803e3d6000fd5b50505050565b6000546001600160a01b031681565b6001546001600160a01b031681565b60e01c90565b600060443d10156102e257610375565b600481823e6308c379a06102f682516102cc565b1461030057610375565b6040513d600319016004823e80513d67ffffffffffffffff81602484011181841117156103305750505050610375565b8284019250825191508082111561034a5750505050610375565b503d8301602082840101111561036257505050610375565b601f01601f191681016020016040529150505b9056fea2646970667358221220577efd69e9b6bd0aef315ca8b576c73ea45e4fdd661c80354676892187cee1dd64736f6c63430007060033";
const REVERT_DEMO_BYTECODE:&str = "0x608060405234801561001057600080fd5b50610109806100206000396000f3fe6080604052348015600f57600080fd5b506004361060325760003560e01c80635e56f344146037578063a814827114603f575b600080fd5b603d6045565b005b603d6098565b306001600160a01b031663a81482716040518163ffffffff1660e01b8152600401600060405180830381600087803b158015607f57600080fd5b505af11580156092573d6000803e3d6000fd5b50505050565b6040805162461bcd60e51b815260206004820152600b60248201526a5265766572742064656d6f60a81b604482015290519081900360640190fdfea2646970667358221220ec2b7033a5b157556e539f3bcae34ab87defd9acac77633153af96a8be1644b364736f6c63430007060033";

/// Test nested revert handling with try-catch mechanism
///
/// Verifies:
/// - Proper handling of nested contract calls
/// - Try-catch error handling
/// - Error propagation in multicall context
/// - Trace address tracking
#[tokio::test(flavor = "multi_thread")]
async fn test_nested_revert_with_try_catch() -> anyhow::Result<()> {
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_tracer(ETH_RPC_URL, inspector).await?;

    // get current nonce to calculate contract address
    let current_account = evm.db().basic(SENDER).unwrap().unwrap();
    let nonce = current_account.nonce;
    let revert_demo_address = SENDER.create(nonce);
    let owner_demo_address = SENDER.create(nonce + 1);

    // 1. deploy RevertDemo contract
    let tx0 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Create,
        value: U256::ZERO,
        data: hex::decode(REVERT_DEMO_BYTECODE).unwrap().into(),
    };

    // 2. deploy OwnerDemo contract
    let tx1 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Create,
        value: U256::ZERO,
        data: hex::decode(OWNER_DEMO_BYTECODE).unwrap().into(),
    };

    // 3. call setRevertDemo to set revert_address
    let data = OwnerDemo::setRevertDemoCall {
        _revert_address: revert_demo_address,
    }
    .abi_encode();
    let tx2 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Call(owner_demo_address),
        value: U256::ZERO,
        data: data.into(),
    };

    // 4. call revert_demo_multi to trigger two calls
    let data = OwnerDemo::revert_demo_multiCall {}.abi_encode();
    let tx3 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Call(owner_demo_address),
        value: U256::ZERO,
        data: data.into(),
    };

    // execute all transactions
    let results = evm
        .trace_transactions(SimulationBatch {
            is_stateful: true,
            transactions: vec![tx0, tx1, tx2, tx3],
        })
        .into_iter()
        .map(|v| v.unwrap())
        .collect::<Vec<_>>();

    // verify results
    assert_eq!(results.len(), 4, "Each tx should have an ExecutionResult");

    // verify transaction failed
    assert!(!results[3].0.is_success(), "Tx should be failed");
    // verify error info (error from second call)
    match &results[3].0.output() {
        Some(output) => {
            let reason = parse_custom_error(output).unwrap();
            assert_eq!(reason, "Revert demo", "Should have correct revert reason");
        }
        _ => panic!("Expected revert failure"),
    }

    // verify call chain
    let top_traces = &results[3].1.call_trace;
    assert!(top_traces.is_some(), "Tx should have one top-level traces");
    let top_traces = top_traces.as_ref().unwrap();
    assert!(
        top_traces.trace_address.is_empty(),
        "Top-level trace should have empty trace_address"
    );
    assert_eq!(
        top_traces.subtraces.len(),
        2,
        "Top-level trace should have two subtraces"
    );

    // verify first call (catched by try-catch)
    let first_subtrace = &top_traces.subtraces[0];
    assert_eq!(
        first_subtrace.trace_address,
        vec![0],
        "First subtrace should have trace_address [0]"
    );
    assert!(
        !first_subtrace.status.is_success(),
        "First subtrace should fail"
    );

    // verify last call (cause actual revert)
    let last_trace = &top_traces.subtraces[1];
    assert_eq!(
        last_trace.trace_address,
        vec![1],
        "Last call should have trace_address [1]"
    );
    assert!(!last_trace.status.is_success(), "Last call should fail");

    // verify final call (caught by try-catch)
    let final_subtrace = &last_trace.subtraces[0];
    assert_eq!(
        final_subtrace.trace_address,
        vec![1, 0],
        "Final subtrace should have trace_address [1,0]"
    );
    assert!(
        !final_subtrace.status.is_success(),
        "Final subtrace should fail"
    );
    assert!(
        final_subtrace.error_origin,
        "Final subtrace should be error origin"
    );

    // verify error trace
    let error_trace_address = results[3].1.error_trace_address.as_ref().unwrap();
    assert_eq!(
        *error_trace_address,
        vec![1, 0],
        "Error trace should be from the second call"
    );

    Ok(())
}

/// Test nested revert handling in multicall context
///
/// Verifies:
/// - Error propagation in nested calls
/// - Trace address tracking
/// - Error origin identification
#[tokio::test(flavor = "multi_thread")]
async fn test_nested_revert_with_multicall() -> anyhow::Result<()> {
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_tracer(ETH_RPC_URL, inspector).await?;

    // get current nonce to calculate contract address
    let current_account = evm.db().basic(SENDER).unwrap().unwrap();

    let nonce = current_account.nonce;
    let revert_demo_address = SENDER.create(nonce);
    let owner_demo_address = SENDER.create(nonce + 1);

    // 1. deploy ReverDemo contract
    let tx0 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Create,
        value: U256::ZERO,
        data: hex::decode(REVERT_DEMO_BYTECODE).unwrap().into(),
    };

    // 2. deploy OwnerDemo contract
    let tx1 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Create,
        value: U256::ZERO,
        data: hex::decode(OWNER_DEMO_BYTECODE).unwrap().into(),
    };

    // 3. call setRevertDemo to set revert_address
    let data = OwnerDemo::setRevertDemoCall {
        _revert_address: revert_demo_address,
    }
    .abi_encode();
    let tx2 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Call(owner_demo_address),
        value: U256::ZERO,
        data: data.into(),
    };

    // 4. call revert_demo to trigger nested call failure
    let data = OwnerDemo::revert_demoCall {}.abi_encode();
    let tx3 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Call(owner_demo_address),
        value: U256::ZERO,
        data: data.into(),
    };

    // execute all transactions
    let results = evm
        .trace_transactions(SimulationBatch {
            is_stateful: true,
            transactions: vec![tx0, tx1, tx2, tx3],
        })
        .into_iter()
        .map(|v| v.unwrap())
        .collect::<Vec<_>>();

    // verify results
    assert_eq!(results.len(), 4, "Each tx should have one execution result");

    // verify transaction failed
    assert!(!results[3].0.is_success(), "Tx3 should be failed");

    // verify error info
    match &results[3].0.output() {
        Some(output) => {
            let reason = parse_custom_error(output).unwrap();
            assert_eq!(reason, "Revert demo", "Should have correct revert reason");
        }
        _ => panic!("Expected revert failure"),
    }

    // verify call chain
    let top_traces = &results[3].1.call_trace;
    assert!(top_traces.is_some(), "Tx should have one top-level traces");
    let top_traces = top_traces.as_ref().unwrap();
    assert!(
        top_traces.trace_address.is_empty(),
        "Top-level trace should have empty trace_address"
    );
    assert_eq!(
        top_traces.subtraces.len(),
        1,
        "Top-level trace should have two subtraces"
    );

    let error_trace_address = results[3].1.error_trace_address.as_ref().unwrap();
    assert_eq!(
        *error_trace_address,
        vec![0, 0],
        "Error trace should be the latest call"
    );

    let mid_trace = &top_traces.subtraces[0];
    assert_eq!(
        mid_trace.trace_address,
        vec![0],
        "Mid trace should have trace_address [0]"
    );
    assert!(!mid_trace.status.is_success(), "Mid trace should be failed");
    assert!(
        !mid_trace.error_origin,
        "Mid trace should not be error origin"
    );

    let final_trace = &mid_trace.subtraces[0];
    assert_eq!(
        final_trace.trace_address,
        vec![0, 0],
        "Final trace should have trace_address [0,0]"
    );
    assert!(
        !final_trace.status.is_success(),
        "Final trace should be failed"
    );
    assert!(
        final_trace.error_origin,
        "Final trace should be error origin"
    );

    Ok(())
}

/// Test nested revert handling without multicall
///
/// Verifies:
/// - Individual transaction execution
/// - Error handling in standalone context
/// - Trace collection and verification
#[tokio::test(flavor = "multi_thread")]
async fn test_nested_revert_without_multicall() -> anyhow::Result<()> {
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_tracer(ETH_RPC_URL, inspector).await?;

    // get current nonce to calculate contract address
    let current_account = evm.db().basic(SENDER).unwrap().unwrap();
    let nonce = current_account.nonce;
    let revert_demo_address = SENDER.create(nonce);
    let owner_demo_address = SENDER.create(nonce + 1);

    // 1. deploy ReverDemo contract
    let tx0 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Create,
        value: U256::ZERO,
        data: hex::decode(REVERT_DEMO_BYTECODE).unwrap().into(),
    };

    // 2. deploy OwnerDemo contract
    let tx1 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Create,
        value: U256::ZERO,
        data: hex::decode(OWNER_DEMO_BYTECODE).unwrap().into(),
    };

    // 3. call setRevertDemo to set revert_address
    let data = OwnerDemo::setRevertDemoCall {
        _revert_address: revert_demo_address,
    }
    .abi_encode();
    let tx2 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Call(owner_demo_address),
        value: U256::ZERO,
        data: data.into(),
    };

    // 4. call revert_demo to trigger nested call failure
    let data = OwnerDemo::revert_demoCall {}.abi_encode();
    let tx3 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Call(owner_demo_address),
        value: U256::ZERO,
        data: data.into(),
    };

    // execute all transactions
    let results = evm
        .trace_transactions(SimulationBatch {
            is_stateful: true,
            transactions: vec![tx0, tx1, tx2, tx3],
        })
        .into_iter()
        .map(|v| v.unwrap())
        .collect::<Vec<_>>();

    // verify results
    assert_eq!(
        results.len(),
        4,
        "Should have results for all four transactions"
    );

    // verify first three calls succeed
    assert!(
        results[0].0.is_success(),
        "ReverDemo deployment should succeed"
    );
    assert!(
        results[1].0.is_success(),
        "OwnerDemo deployment should succeed"
    );
    assert!(
        results[2].0.is_success(),
        "setRevertDemo call should succeed"
    );

    // verify last call failed
    assert!(!results[3].0.is_success(), "revert_demo call should fail");

    // verify error info
    match &results[3].0.output() {
        Some(output) => {
            let reason = parse_custom_error(output).unwrap();
            assert_eq!(reason, "Revert demo", "Should have correct revert reason");
        }
        _ => panic!("Expected revert failure"),
    }

    // verify call chain

    let top_trace = &results[3].1.call_trace.as_ref().unwrap();
    assert_eq!(top_trace.subtraces.len(), 1, "Should have one subtrace");
    assert_eq!(top_trace.from, SENDER);
    assert_eq!(top_trace.to, owner_demo_address);
    assert!(!top_trace.status.is_success(), "Top trace should be failed");
    assert!(
        top_trace.trace_address.is_empty(),
        "Top trace should have empty trace_address"
    );
    assert!(
        !top_trace.error_origin,
        "Top trace should not be error origin"
    );

    let sub_trace = &top_trace.subtraces[0];
    assert_eq!(sub_trace.from, owner_demo_address);
    assert_eq!(sub_trace.to, revert_demo_address);
    assert_eq!(
        sub_trace.trace_address,
        vec![0],
        "Subtrace should have trace_address [0]"
    );
    assert!(!sub_trace.status.is_success(), "Subtrace should be failed");
    assert!(
        !sub_trace.error_origin,
        "Subtrace should not be error origin"
    );

    let final_trace = &sub_trace.subtraces[0];
    assert_eq!(final_trace.from, revert_demo_address);
    assert_eq!(final_trace.to, revert_demo_address);
    assert_eq!(
        final_trace.trace_address,
        vec![0, 0],
        "Subtrace should have trace_address [0]"
    );
    assert!(
        !final_trace.status.is_success(),
        "Subtrace should be failed"
    );
    assert!(final_trace.error_origin, "Subtrace should  be error origin");

    // verify error trace
    let error_trace = results[3].1.error_trace_address.as_ref().unwrap();
    assert_eq!(
        *error_trace,
        vec![0, 0],
        "Error_trace should be same as final_trace"
    );

    Ok(())
}

/// Test multicall execution with error handling
///
/// Verifies:
/// - Batch transaction processing
/// - Error handling in multicall context
/// - Transaction ordering and state changes
#[tokio::test(flavor = "multi_thread")]
async fn test_multicall_with_error() -> anyhow::Result<()> {
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_tracer(ETH_RPC_URL, inspector).await?;
    // get current nonce to calculate contract address
    let current_account = evm.db().basic(SENDER).unwrap().unwrap();
    let nonce = current_account.nonce;
    let expected_contract_address = SENDER.create(nonce);

    // 1. deploy OwnerDemo contract
    let tx0 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Create,
        value: U256::ZERO,
        data: hex::decode(OWNER_DEMO_BYTECODE).unwrap().into(),
    };

    // 2. non-owner attempt to set owner (will fail)
    let data = OwnerDemo::setOwnerCall {
        _owner: DEAD_ADDRESS,
    }
    .abi_encode();
    let tx1 = SimulationTx {
        caller: CAFE_ADDRESS,
        transact_to: TxKind::Call(expected_contract_address),
        value: U256::ZERO,
        data: data.clone().into(),
    };

    // 3. owner set new owner transaction (will succeed)
    let tx2 = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Call(expected_contract_address),
        value: U256::ZERO,
        data: data.clone().into(),
    };

    // execute batch transactions
    let results = evm
        .trace_transactions(SimulationBatch {
            is_stateful: true,
            transactions: vec![tx0, tx1, tx2],
        })
        .into_iter()
        .map(|v| v.unwrap())
        .collect::<Vec<_>>();

    // verify results
    assert_eq!(results.len(), 3, "Each tx should have one execution result");
    let result = &results[1].0;

    match result.output() {
        Some(output) => {
            let reason = parse_custom_error(output).unwrap();
            assert_eq!(
                reason, "Only the owner can set the owner",
                "Should fail with correct revert reason"
            );
        }
        _ => panic!("Expected revert failure"),
    }

    // verify error trace

    let error_trace = results[1].1.call_trace.as_ref().unwrap();
    assert_eq!(
        error_trace.from, CAFE_ADDRESS,
        "Error should come from CAFE_ADDRESS call"
    );
    assert_eq!(
        error_trace.to, expected_contract_address,
        "Error should be in contract call"
    );
    assert!(
        error_trace.trace_address.is_empty(),
        "Error should be in the top transaction"
    );

    Ok(())
}

/// Test contract creation and deployment
///
/// Verifies:
/// - Contract deployment process
/// - Address prediction
/// - Creation trace collection
#[tokio::test(flavor = "multi_thread")]
async fn test_create_contract() {
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_tracer(ETH_RPC_URL, inspector)
        .await
        .unwrap();

    let sender = address!("b20a608c624Ca5003905aA834De7156C68b2E1d0");
    let current_account = evm.db().basic(sender).unwrap().unwrap();
    let nonce = current_account.nonce;
    let expected_contract_address = sender.create(nonce);

    let data = hex::decode(OWNER_DEMO_BYTECODE).unwrap();

    let tx0 = SimulationTx {
        caller: sender,
        transact_to: TxKind::Create,
        value: U256::ZERO,
        data: data.into(),
    };
    let results = evm
        .trace_transactions(SimulationBatch {
            is_stateful: false,
            transactions: vec![tx0],
        })
        .into_iter()
        .map(|v| v.unwrap())
        .collect::<Vec<_>>();

    assert_eq!(results.len(), 1, "Should have results for one transaction");
    let result = &results[0].0;
    assert!(result.is_success(), "Contract creation should succeed");
    // verify contract creation output
    let call_trace = &results[0].1.call_trace.as_ref().unwrap();
    assert_eq!(call_trace.from, sender, "Creator should match");
    assert_eq!(
        call_trace.to, expected_contract_address,
        "Contract address should match"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_stateful_and_stateless_call_trace() {
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_tracer(ETH_RPC_URL, inspector)
        .await
        .unwrap();

    let sender = address!("b20a608c624Ca5003905aA834De7156C68b2E1d0");
    let current_account = evm.db().basic(sender).unwrap().unwrap();
    let nonce = current_account.nonce;
    let expected_contract_address = sender.create(nonce);
    let next_contract_address = sender.create(nonce + 1);

    let data = hex::decode(OWNER_DEMO_BYTECODE).unwrap();

    let tx0 = SimulationTx {
        caller: sender,
        transact_to: TxKind::Create,
        value: U256::ZERO,
        data: data.clone().into(),
    };
    let tx1 = SimulationTx {
        caller: sender,
        transact_to: TxKind::Create,
        value: U256::ZERO,
        data: data.into(),
    };

    let results = evm
        .trace_transactions(SimulationBatch {
            is_stateful: false,
            transactions: vec![tx0.clone(), tx1.clone()],
        })
        .into_iter()
        .map(|v| v.unwrap())
        .collect::<Vec<_>>();

    assert_eq!(results.len(), 2, "Should have results for two transactions");
    assert!(
        results[0].0.is_success(),
        "Contract creation should succeed"
    );
    assert!(results[1].0.is_success(), "setOwner should succeed");
    let deploy_call_tx0 = results[0].1.call_trace.as_ref().unwrap();
    assert_eq!(deploy_call_tx0.from, sender, "Creator should match");
    assert_eq!(
        deploy_call_tx0.to, expected_contract_address,
        "Contract address should match"
    );

    let deploy_call_tx1 = results[1].1.call_trace.as_ref().unwrap();
    assert_eq!(deploy_call_tx1.from, sender, "Creator should match");
    assert_eq!(
        deploy_call_tx1.to, expected_contract_address,
        "Contract address should match"
    );

    let results = evm
        .trace_transactions(SimulationBatch {
            is_stateful: true,
            transactions: vec![tx0.clone(), tx1.clone()],
        })
        .into_iter()
        .map(|v| v.unwrap())
        .collect::<Vec<_>>();

    assert_eq!(results.len(), 2, "Should have results for two transactions");
    assert!(
        results[0].0.is_success(),
        "Contract creation should succeed"
    );
    assert!(results[1].0.is_success(), "setOwner should succeed");
    let deploy_call_tx0 = results[0].1.call_trace.as_ref().unwrap();
    assert_eq!(deploy_call_tx0.from, sender, "Creator should match");
    assert_eq!(
        deploy_call_tx0.to, expected_contract_address,
        "Contract address should match"
    );

    let deploy_call_tx1 = results[1].1.call_trace.as_ref().unwrap();
    assert_eq!(deploy_call_tx1.from, sender, "Creator should match");
    assert_eq!(
        deploy_call_tx1.to, next_contract_address,
        "Contract address should match"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_wth_ws() -> anyhow::Result<()> {
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_tracer(ETH_RPC_URL, inspector).await?;

    // check initial state
    let cafe_balance_before = evm.db().basic(CAFE_ADDRESS).unwrap().unwrap().balance;
    let dead_balance_before = evm.db().basic(DEAD_ADDRESS).unwrap().unwrap().balance;
    assert_eq!(
        cafe_balance_before,
        U256::ZERO,
        "CAFE initial balance should be 0"
    );
    assert_eq!(
        dead_balance_before,
        U256::ZERO,
        "DEAD initial balance should be 0"
    );

    // define transfer amounts
    let transfer1_amount = U256::from(100000000000000000u64); // 0.1 ETH
    let transfer2_amount = U256::from(60000000000000000u64); // 0.06 ETH

    let txs = SimulationBatch {
        is_stateful: true,
        transactions: vec![
            SimulationTx {
                caller: SENDER,
                transact_to: TxKind::Call(CAFE_ADDRESS),
                value: transfer1_amount,
                data: vec![].into(),
            },
            SimulationTx {
                caller: CAFE_ADDRESS,
                transact_to: TxKind::Call(DEAD_ADDRESS),
                value: transfer2_amount,
                data: vec![].into(),
            },
        ],
    };

    let results = evm
        .trace_transactions(txs)
        .into_iter()
        .map(|v| v.unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results.len(),
        2,
        "Should have results for both transactions"
    );

    // verify first tx
    let result0 = &results[0];
    assert!(result0.0.is_success(), "First tx should succeed");
    let transfer1 = &result0.1.asset_transfers[0];
    assert_eq!(transfer1.from, SENDER);
    assert_eq!(transfer1.to, Some(CAFE_ADDRESS));
    assert_eq!(transfer1.value, transfer1_amount);
    assert!(transfer1.is_native_token());

    // verify second transfer
    let result1 = &results[1];
    assert!(result1.0.is_success(), "Second tx should succeed");
    let transfer2 = &result1.1.asset_transfers[0];
    assert_eq!(transfer2.from, CAFE_ADDRESS);
    assert_eq!(transfer2.to, Some(DEAD_ADDRESS));
    assert_eq!(transfer2.value, transfer2_amount);
    assert!(transfer2.is_native_token());

    // verify final state
    let cafe_balance_after = evm.db().basic(CAFE_ADDRESS).unwrap().unwrap().balance;
    let dead_balance_after = evm.db().basic(DEAD_ADDRESS).unwrap().unwrap().balance;

    // calculate expected balance
    let expected_cafe_balance = transfer1_amount - transfer2_amount;
    assert_eq!(
        cafe_balance_after, expected_cafe_balance,
        "CAFE should have 0.04 ETH left"
    );
    assert_eq!(
        dead_balance_after, transfer2_amount,
        "DEAD should have received 0.06 ETH"
    );

    Ok(())
}
