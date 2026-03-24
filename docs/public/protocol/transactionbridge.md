# TransactionBridge

{% include "../../../.gitbook/includes/toc-payy-transactions.md" %}

The TransactionBridge is a genesis deployed contract for Payy transaction submission.
It exposes EIP-712-compatible transaction hashing for tooling, while `submit` accepts only direct sender submissions (`msg.sender == txn.from`).

Batches have two execution modes:

- For atomic batches (`requireSuccess = true`), every call must succeed. If any subcall fails, the entire transaction reverts, earlier successful subcalls are rolled back, no `TxnCallResult`, `TxnProcessed`, or `TxnRejected` logs persist, and the nonce/status remain unchanged. An atomic batch failure is therefore a reverted submission, not a `TxnRejected` outcome.
- For best-effort batches (`requireSuccess = false`), the bridge still executes every call in order and emits `TxnCallResult` for each one. The transaction still completes in the processed lifecycle state, and `TxnProcessed.success` flips to `false` if any subcall fails. Those partial failures are reflected only in the per-call results plus `TxnProcessed.success = false`; they do not emit `TxnRejected`.

The following shows the Solidity interface for the TransactionBridge:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface ITransactionBridge {
    // ----------------------------
    // Structs
    // ----------------------------

    struct Call {
        address to;
        uint256 value;
        uint256 gasLimit;
        bytes data;
        // Optional authoriser signature (EIP-712 over this Txn without signature field)
        bytes signature;
    }

    // How the fee is paid (currency + caps)
    struct FeePayment {
        // 0 = Native, 1 = ERC20
        uint8 currency;
        address token;              // ERC-20 token address if currency == 1
        uint256 maxFee;             // cap in native units (or token units if ERC-20)
        uint256 maxPriorityFee;     // optional tip cap (native units semantics)
    }

    // Who pays and why they're allowed to pay (sponsorship or self-pay)
    struct FeePayer {
        // 0 = Self (txn.from pays), 1 = Sponsored
        uint8 mode;

        // The account responsible for paying the fee (EOA or Contract)
        address payer;

        // Sponsorship authorisation data:
        // - For EOA-signed vouchers: abi-encoded SponsorVoucher + signature
        // - For contract approvals: abi-encoded params used by the payer contract's approve function
        bytes authData;

        // Discriminator for how to validate authData:
        // 0 = None (self-pay), 1 = EOA voucher, 2 = Contract approval
        uint8 authKind;
    }

    struct Delegation {
        bool enabled;
        address delegator;
        uint8 scopeType;            // 0=None,1=Contract,2=FunctionSelector,3=Wildcard
        address scopeTarget;
        bytes4 functionSelector;
        uint256 notBefore;
        uint256 notAfter;
        uint256 revocationNonce;
    }

    struct Schedule {
        uint256 notBefore;
        uint256 notAfter;
    }

    struct NonceSpace {
        bytes32 key;
        uint256 nonce;
    }

    struct Txn {
        address from;
        Call[] calls;
        FeePayment feePayment;      // currency + caps
        FeePayer feePayer;          // who pays + sponsorship auth
        Schedule schedule;
        Delegation delegation;
        NonceSpace nonceSpace;
        bytes32 salt;
        bool requireSuccess;
        bytes metadata;
        uint256 chainId;
        uint256 expiry;
    }

    struct CancelTxn {
        address from;
        bytes32 txnHash;
        NonceSpace nonceSpace;
        uint256 chainId;
        uint256 expiry;
        bytes reason;
    }

    // ----------------------------
    // Events
    // ----------------------------

    event TxnSubmitted(
        bytes32 indexed txnHash,
        address indexed from,
        address indexed relayer,
        bytes32 nonceSpaceKey,
        uint256 nonce
    );

    event TxnProcessed(
        bytes32 indexed txnHash,
        address indexed from,
        bool success,
        uint256 totalGasUsed,
        uint256 feePaid,
        address feeToken,
        address payer
    );

    event TxnCallResult(
        bytes32 indexed txnHash,
        uint256 index,
        address to,
        bool success,
        uint256 gasUsed,
        bytes returnData
    );

    event TxnRejected(
        bytes32 indexed txnHash,
        address indexed from,
        uint8 reasonCode,
        bytes reasonData
    );

    event TxnCancelled(
        bytes32 indexed txnHash,
        address indexed from,
        bytes32 nonceSpaceKey,
        uint256 nonce
    );

    // ----------------------------
    // Core functions
    // ----------------------------

    function submit(Txn calldata txn) external payable;

    // ----------------------------
    // Helpers
    // ----------------------------

    function eip712DomainSeparator() external view returns (bytes32);
    function txnHash(Txn calldata txn) external view returns (bytes32);
    function nextNonce(address from, bytes32 key) external view returns (uint256);
    function getStatus(bytes32 txnHash) external view returns (uint8 status);
}
```
