// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {IERC1271} from "@openzeppelin/contracts/interfaces/IERC1271.sol";
import {IERC165} from "@openzeppelin/contracts/utils/introspection/IERC165.sol";

/// Minimal meta-transaction smart account to be used as an EIP-7702 delegate.
/// It verifies EIP-712 signatures from the account owner (the EOA's key) and executes calls.
/// Now also implements ERC-1271 so signatures validate when other contracts see code at this address.
contract Eip7702SimpleAccount is IERC1271, IERC165 {
    // =========================
    // EIP-712 domain constants
    // =========================
    bytes32 private constant EIP712DOMAIN_TYPEHASH =
        keccak256(
            "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
        );
    bytes32 private constant NAME_HASH = keccak256("Eip7702SimpleAccount");
    bytes32 private constant VERSION_HASH = keccak256("1");

    // =========================
    // Batched Execute structs
    // =========================
    bytes32 private constant CALL_TYPEHASH =
        keccak256("Call(address target,uint256 value,bytes data)");
    bytes32 private constant EXECUTE_MANY_TYPEHASH =
        keccak256(
            "ExecuteMany(bytes32[] calls,uint256 nonce,uint256 validAfter,uint256 validUntil)"
        );

    struct Call {
        address target;
        uint256 value;
        bytes data;
    }

    // Deterministic nonce tracking keyed by uint256 (no conversion needed by callers)
    mapping(uint256 => bool) public usedNonces;

    /// Emitted when a meta-transaction with a given nonce is accepted and executed
    event NonceUsed(uint256 indexed nonce);

    constructor() {}

    // ============
    // EIP-712
    // ============
    function domainSeparator() public view returns (bytes32) {
        return
            keccak256(
                abi.encode(
                    EIP712DOMAIN_TYPEHASH,
                    NAME_HASH,
                    VERSION_HASH,
                    block.chainid,
                    address(this)
                )
            );
    }

    function _hashCall(Call calldata c) internal pure returns (bytes32) {
        return
            keccak256(
                abi.encode(CALL_TYPEHASH, c.target, c.value, keccak256(c.data))
            );
    }

    /// Hash a batch (ExecuteMany) per EIP-712.
    function _hashExecuteMany(
        Call[] calldata calls,
        uint256 _nonce,
        uint256 validAfter,
        uint256 validUntil
    ) internal view returns (bytes32) {
        bytes32 callsHash = hashCalls(calls);
        bytes32 structHash = keccak256(
            abi.encode(
                EXECUTE_MANY_TYPEHASH,
                callsHash,
                _nonce,
                validAfter,
                validUntil
            )
        );
        return
            keccak256(
                abi.encodePacked("\x19\x01", domainSeparator(), structHash)
            );
    }

    /// Compute the hash of an array of Calls exactly as the off-chain signer does.
    function hashCalls(Call[] calldata calls) public pure returns (bytes32) {
        bytes32[] memory callHashes = new bytes32[](calls.length);
        for (uint256 i = 0; i < calls.length; i++) {
            callHashes[i] = _hashCall(calls[i]);
        }
        return keccak256(abi.encodePacked(callHashes));
    }

    /// Execute a batch of calls authorized by a user signature (the EOA's key).
    function executeMeta(
        Call[] calldata calls,
        uint256 _nonce,
        uint256 validAfter,
        uint256 validUntil,
        bytes calldata signature
    ) external {
        require(!usedNonces[_nonce], "nonce already used");
        require(block.timestamp >= validAfter, "authorization not yet valid");
        require(block.timestamp <= validUntil, "authorization expired");

        bytes32 digest = _hashExecuteMany(
            calls,
            _nonce,
            validAfter,
            validUntil
        );
        address signer = ECDSA.recover(digest, signature);

        // Under EIP-7702, this code executes at the EOA's address, so the expected signer is address(this).
        require(signer == address(this), "bad signature");

        usedNonces[_nonce] = true;
        emit NonceUsed(_nonce);

        for (uint256 i = 0; i < calls.length; i++) {
            (bool ok, bytes memory ret) = calls[i].target.call{
                value: calls[i].value
            }(calls[i].data);
            require(ok, string(ret));
        }
    }

    function isNonceUsed(uint256 _nonce) external view returns (bool) {
        return usedNonces[_nonce];
    }

    // ======================
    // ERC-1271 (signature validation for contract accounts)
    // ======================

    // Magic return values
    bytes4 private constant _ERC1271_MAGICVALUE =
        IERC1271.isValidSignature.selector; // 0x1626ba7e
    bytes4 private constant _ERC1271_INVALID = 0xffffffff; // any value != magic is invalid
    // Legacy (v0) "bytes,bytes" magic value for broader compatibility
    bytes4 private constant _ERC1271_MAGICVALUE_BYTES = 0x20c13b0b; // bytes4(keccak256("isValidSignature(bytes,bytes)"))

    /// @notice ERC-1271 final: validate a signature over a 32-byte digest.
    /// @dev MUST NOT revert on bad signatures — return a non-magic value instead.
    function isValidSignature(
        bytes32 hash,
        bytes calldata signature
    ) external view override returns (bytes4) {
        (address recovered, ECDSA.RecoverError err, ) = ECDSA.tryRecover(
            hash,
            signature
        );
        if (err == ECDSA.RecoverError.NoError && recovered == address(this)) {
            return _ERC1271_MAGICVALUE;
        }
        return _ERC1271_INVALID;
    }

    /// @notice ERC-1271 v0 variant: validate a signature over arbitrary bytes (hashed as keccak256).
    /// @dev Returned magic value differs from the final ERC-1271 variant by design.
    function isValidSignature(
        bytes calldata data,
        bytes calldata signature
    ) external view returns (bytes4) {
        (address recovered, ECDSA.RecoverError err, ) = ECDSA.tryRecover(
            keccak256(data),
            signature
        );
        if (err == ECDSA.RecoverError.NoError && recovered == address(this)) {
            return _ERC1271_MAGICVALUE_BYTES;
        }
        return _ERC1271_INVALID;
    }

    // ======================
    // ERC-165 (optional, helps feature detection)
    // ======================

    function supportsInterface(
        bytes4 interfaceId
    ) external pure override returns (bool) {
        // ERC165 itself OR ERC1271
        return
            interfaceId == type(IERC165).interfaceId ||
            interfaceId == type(IERC1271).interfaceId; // 0x1626ba7e
    }

    /// Accept direct native token transfers when this contract is delegated via EIP-7702.
    receive() external payable {}
}
