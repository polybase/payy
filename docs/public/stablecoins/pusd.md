# PUSD

PUSD is the native stablecoin for Payy. A US treasuries backed USD stablecoin that is highly safe and compliant. PUSD is issued from the US and is compliant with the Genius Act.

PUSD is the native token for Payy (similar to ETH on Ethereum), allowing simpler user experiences when dealing with gas payments, as no conversion is necessary when displaying gas prices.

| Key      | Value       |
| -------- | ----------- |
| Name     | Payy USD    |
| Symbol   | PUSD        |
| Decimals | 18 decimals |
| Permit   | ✅           |

### ERC-20 Interface

PUSD is available as both an ERC-20 and as the native token, with balances automatically synced across both states. This means applications no longer need to special case the native token.

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title PUSD ERC-20 Interface with EIP-2612 Permit
/// @notice ERC-20 + Metadata + EIP-2612 permit + EIP-712 domain accessors
interface IPUSD {
    // ------------------------------------------------------------
    // Events (ERC-20)
    // ------------------------------------------------------------
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    // ------------------------------------------------------------
    // ERC-20 Standard
    // ------------------------------------------------------------
    function totalSupply() external view returns (uint256);

    function balanceOf(address account) external view returns (uint256);

    function allowance(address owner, address spender) external view returns (uint256);

    function approve(address spender, uint256 value) external returns (bool);

    function transfer(address to, uint256 value) external returns (bool);

    function transferFrom(address from, address to, uint256 value) external returns (bool);

    // ------------------------------------------------------------
    // ERC-20 Metadata (EIP-20 optional)
    // ------------------------------------------------------------
    function name() external view returns (string memory);

    function symbol() external view returns (string memory);

    function decimals() external view returns (uint8);

    // ------------------------------------------------------------
    // EIP-2612 Permit
    // ------------------------------------------------------------
    /// @notice Sets `value` as the allowance of `spender` over `owner`'s tokens, via signature.
    /// @dev See EIP-2612. Should revert on invalid signature, expired deadline, or if owner == address(0).
    ///      Implementations may support chain-specific nonces and domain separators (EIP-712).
    /// @param owner The token owner granting allowance
    /// @param spender The spender being approved
    /// @param value The allowance value
    /// @param deadline Timestamp after which the permit is invalid
    /// @param v secp256k1 signature v
    /// @param r secp256k1 signature r
    /// @param s secp256k1 signature s
    function permit(
        address owner,
        address spender,
        uint256 value,
        uint256 deadline,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external;

    /// @notice Current nonce for `owner` used in EIP-2612 permit signatures
    function nonces(address owner) external view returns (uint256);

    /// @notice EIP-712 domain separator used in the encoding of the permit signature
    function DOMAIN_SEPARATOR() external view returns (bytes32);
}

```
