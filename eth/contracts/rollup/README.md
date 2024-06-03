## Upgrades

The rollup contract uses a proxy pattern to allow for upgrades.
If you want to make a change to the rollup contract, you need to create a new version of the contract that inherits from the old one.
The storage layout of the new contract must be backwards compatible with the old one, meaning you should not change the variables or their order in previous versions of the contract.
Overriding old functions in a new version of the contract is fine, but keep in mind that if you change the parameter types, you will create a new function and the old one will still be callable.

Example:

```solidity
import "./RollupV1.sol";

// The order of the inherited contracts is important to ensure the storage layout is unchanged.
contract RollupV2 is RollupV1, AnyOtherContracts {
    uint256 public newVariable;

    function initializeV2() public reinitializer(2) {
        version = 2;
        newVariable = 1;
    }

    function newFunction() public {}
}
```

The `initializeV2` function would need to be called by the proxy contract to perform the upgrade.

Add the new version of the contract to both `deploy.ts` and `upgrade.ts`.

### Rollup USDC Balance

Because the proxy uses `delegateCall`, the contract's USDC balance persists across upgrades, since in calls to USDC, the `msg.sender` and `this` is the proxy contract, not the implementer rollup contract.

Proxy (msg.sender) (->) RollupV1 (still Proxy msg.sender) -> USDC
