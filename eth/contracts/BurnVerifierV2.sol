// SPDX-License-Identifier: MIT
// Originally copied from https://github.com/scroll-tech/scroll/blob/ff380141a8cbcc214dc65f17ffa44faf4be646b6/contracts/src/libraries/verifier/ZkEvmVerifierV1.sol

pragma solidity ^0.8.20;

import "./Verifier.sol";

// import "hardhat/console.sol";

// solhint-disable no-inline-assembly

contract BurnVerifierV2 is Verifier {
    /**********
     * Errors *
     **********/

    /// @dev Thrown when aggregate zk proof verification is failed.
    error VerificationFailed();

    /*************
     * Constants *
     *************/

    /// @notice The address of highly optimized plonk verifier contract.
    address public immutable plonkVerifier;

    /***************
     * Constructor *
     ***************/

    constructor(address _verifier) {
        plonkVerifier = _verifier;
    }

    /*************************
     * Public View Functions *
     *************************/

    function verify(
        bytes calldata proof,
        // Start of instances. Be careful reordering these because of the `calldatacopy` below
        // [kind, addr, nullifier, value, source, root]
        bytes32[6] calldata instances
    ) external view {
        for (uint256 i = 0; i < 6; i++) {
            requireValidFieldElement(instances[i]);
        }

        address _verifier = plonkVerifier;
        bool success;

        uint instancesLength = 6 * 32; // 32 bytes per input, 6 inputs
        bytes memory data = new bytes(instancesLength + proof.length);

        assembly {
            calldatacopy(add(data, 32), instances, instancesLength)
            calldatacopy(
                add(add(data, 32), instancesLength),
                proof.offset,
                proof.length
            )

            success := staticcall(
                gas(),
                _verifier,
                // start of data
                add(data, 32),
                // length
                mload(data),
                0x00,
                0x00
            )
        }

        if (!success) {
            revert VerificationFailed();
        }
    }
}
