
# TxInspector Design Overview & Open Source License

This document is part of the REVM-Trace project and is released under the same open source licenses as the codebase (MIT/Apache-2.0). You are free to use, modify, and redistribute this document under the terms of those licenses.

# TxInspector: Design Philosophy and Feature Overview

`TxInspector` is the core inspector of REVM-Trace, designed for high-performance EVM transaction simulation, asset flow tracking, and security analysis. Its goals and capabilities include:

- **Full Call Tree Tracing**: Automatically builds a complete call tree, recursively reconstructing every contract call, parent-child relationship, and trace_address path.
- **Accurate from/to Resolution**: Supports Call/Create/DelegateCall and more, precisely distinguishing context address, code address, and target address.
- **DelegateCall Semantics**: Differentiates context address and code address, ensuring correct asset attribution, slot changes, and event ownership in delegatecall scenarios.
- **Global and Per-Call Slot Access Tracking**: Recursively collects all storage slot reads/writes (SlotAccess) for each transaction and call trace node, with type filtering (read/write/all), enabling security analysis and attack forensics.
- **Event and Asset Flow Analysis**: Automatically collects all event logs and ETH/ERC20/NFT transfers, supporting deep nesting and batch processing.
- **High-Performance Concurrent Simulation**: Supports multi-threaded, batch transaction simulation, compatible with AlloyDB and Foundry-fork-db backends.
- **Security & Audit Friendly**: Suitable for attack forensics, Safe wallet transaction inspection, DeFi protocol security analysis, and more.
- **Extensibility & Usability**: Unified API and well-documented, making it easy for secondary development and custom analysis.

## Design Principles

- Call tree as the core: every trace node can be uniquely located (trace_address), supporting slot/event/asset attribution at any depth.
- Separation of call_stack (for path) and address_stack (for from/to resolution).
- All active call_traces are temporarily stored; on handle_end, they are attached to the parent node, with only the root node output at the end, ensuring a complete tree structure.
- DelegateCall only switches code address, not context address; trace nodes must distinguish both.
- Slot changes, events, and asset transfers can be attributed to any trace node for fine-grained security analysis.

## Typical Use Cases

- Reconstructing complex attack paths, privilege escalations, and DelegateCall abuse
- Auditing Safe wallet and multisig contract asset flows and slot changes
- Full-chain asset and state analysis for DeFi batch and nested transactions
- Asset tracing, event attribution, slot change forensics for security and compliance

---


## 1. Call Stack and Trace Node Structure

- `call_stack`: Maintains the current active call stack, with the top being the index of the currently active call_trace. All active call_traces are temporarily stored in a call_traces Vec and are only removed and attached to the parent node's subtraces during handle_end.
    - call_stack also serves as the implementation of trace_address; cloning it gives the current logical path.
- `address_stack`: Records the context address at each call level, used for from/to resolution.
- `call_traces`: Temporary pool for all active call_traces; only the top-level call_trace is retained as the root node for output.
- `trace_address`: The path of each node in the call tree (e.g., [0,1,2] means the 2nd subcall of the 1st subcall of the 0th top-level call). Actually implemented by call_stack, no need for redundant maintenance.

---


## 2. from/to Resolution Logic

- Normal Call/Create:
    - `from`: Taken from the second-to-last element of address_stack (caller).
    - `to`: Taken from the top of address_stack (callee).
    - Before the call, the top of address_stack is the caller; after the call, push the callee.
- DelegateCall:
    - `from`: Remains as the context address (top of address_stack).
    - `to`: Target contract address (the contract being delegatecalled).
    - context address does not change, only code address changes.
    - Trace node needs to record an extra code_address field.

---


## 3. Special Handling for DelegateCall

- DelegateCall does not change the context (storage/balance, etc.), only switches the code address.
- When recording the trace:
    - from = context address (address_stack[-1])
    - to = code address
    - Must distinguish between context address and code address
- The Inspector's call/delegatecall hooks must correctly maintain address_stack and code_address fields.

---


## 4. Call Stack and Parent-Child Relationship Maintenance

- On each call/create/delegatecall:
    - Create a new trace node and push it to call_traces
    - Push the current node index to call_stack
    - Update address_stack (except for DelegateCall)
- On call_end:
    - Pop the current call_stack to get the index of the active call_trace
    - Update call_trace state
    - If there is a parent node (call_stack not empty), remove the current call_trace and attach it to the parent node's subtraces
    - If it is a top-level call, leave it in call_traces as the root node
- This fully reconstructs the call tree structure and supports precise trace_address location for any node

---


## 5. Key Implementation Snippet (Pseudocode)

```rust
// Before call
if is_delegatecall {
    // address_stack unchanged
    code_address = callee;
} else {
    address_stack.push(callee);
    code_address = callee;
}
// Create new trace node
let trace = CallTraceNode { ... };
call_traces.push(trace);
call_stack.push(trace_index); // trace_index = call_traces.len() - 1

// On call_end
if let Some(trace_index) = call_stack.pop() {
    let trace = &mut call_traces[trace_index];
    // ...update trace state...
    if let Some(&parent_index) = call_stack.last() {
        let trace = call_traces.remove(trace_index);
        call_traces[parent_index].subtraces.push(trace);
    }
    // Otherwise, it's a top-level call_trace, leave in call_traces
}
```

---


## 6. Reference Notes

- Core related code is in `inspectors/tx_inspector/inspector.rs` and `trace.rs`.
- For detailed trace node structure and parent-child maintenance logic, refer to the above files.
- To track slot changes within each call, actively record them in the step/step_end hooks.

---


## 7. How to Get the Current call_trace Level (trace_address)

- call_stack is both the active call stack and the implementation of trace_address.
- In the step/step_end hooks, cloning call_stack gives the current logical path, which can be used for slot changes, event attribution, etc.
- address_stack.last() gives the current context address.

---


## 8. Summary and Design Recommendations

- call_traces is the temporary pool for all active call_traces; they are only removed and attached to the parent node during handle_end.
- call_stack is both the active call stack and the implementation of trace_address; no redundant maintenance is needed.
- As long as call_stack push/pop is synchronized with the call_trace tree, any call level can be efficiently located and managed.
- The top-level call_trace has no parent node and is not removed during handle_end; it is output as the root node at the end.

---


## 9. Relationship Between from/to, address_stack, and trace_address

- **from**: Always taken from the top of address_stack (i.e., the current context address), used to determine the caller of the current call.
- **to**: For normal call/create, it's the target address; for delegatecall, it's the bytecode_address.
- **next_caller**: For normal call/create, it's the callee; for delegatecall, it's still from (context address unchanged).
- **address_stack**: Tracks the context address at each level, push/pop synchronized with call depth; only used for from resolution, not for trace_address path.
- **trace_address**: The path of each trace node in the call tree, maintained by call_stack (each level is the index in the parent's subtraces), used to uniquely locate the current call in the entire call tree; not used for from/to resolution.
- **call_stack**: Both the active call stack and the implementation of trace_address; cloning it gives the current logical path.

### Typical Flow
1. from = address_stack.last()
2. to = match scheme { DelegateCall => bytecode_address, _ => target_address }
3. next_caller = match scheme { DelegateCall => from, _ => to }
4. address_stack.push(next_caller)
5. trace_address = parent trace_address + current parent's subtraces.len() (derived from call_stack path)

---

### Additional Notes on Resolution Flow

- **next_caller**: Refers to the new top of address_stack after this call completes, i.e., the from for the next call. It may be used as the from for a nested call, or popped after this call ends to restore the from for the next call at the same level.
    - If there is a nested call, the next level's from = next_caller.
    - If this call ends, address_stack.pop(); the from for the next call at the same level is still the previous context address.
- This ensures that from resolution is always accurate at every level, whether entering recursively or making multiple calls at the same level.

---

This ensures that the semantics of from, to, trace_address, and address_stack are clear and unambiguous, making future maintenance and extension easier.
