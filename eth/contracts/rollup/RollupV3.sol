// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "./RollupV2.sol";

contract RollupV3 is RollupV2 {
    function initializeV3() public reinitializer(3) {
        version = 3;
    }

    // Verify a new block
    function verifyBlock(
        // Transaction[] calldata txns,
        bytes calldata aggrProof,
        bytes32[12] calldata aggrInstances,
        bytes32 oldRoot,
        bytes32 newRoot,
        // 6 utxo x 3 hashes per utxo
        bytes32[18] calldata utxoHashes,
        bytes32 otherHashFromBlockHash,
        uint256 height,
        // uint64 skips,
        Signature[] calldata signatures
    ) public virtual override onlyProver {
        updateValidatorSetIndex(height);
        ValidatorSet storage validatorSet = getValidators();

        require(
            oldRoot == currentRootHash(),
            "Old root does not match the current root"
        );

        // Check mints/burns
        for (uint i = 0; i < 18; i += 3) {
            bytes32 mb = utxoHashes[i + 1];
            bytes32 value = utxoHashes[i + 2];

            if (value == 0) {
                continue;
            }

            if (mints[mb] != 0) {
                require(mints[mb] == uint256(value), "Invalid mint amount");
                delete mints[mb];
                continue;
            }

            if (burns[mb].amount != 0) {
                require(
                    burns[mb].amount == uint256(value),
                    "Invalid burn amount"
                );

                // You cannot transfer to the zero address,
                // otherwise you get 'ERC20: transfer to the zero address'
                if (burns[mb].to != address(0)) {
                    // Perform the transfer to the requested account
                    require(
                        usdc.transfer(burns[mb].to, burns[mb].amount),
                        "Transfer failed"
                    );
                }

                delete burns[mb];
                continue;
            }

            revert("Invalid mint/burn");
        }

        // Check recent roots
        require(
            containsRootHashes(
                [
                    utxoHashes[0],
                    utxoHashes[3],
                    utxoHashes[6],
                    utxoHashes[9],
                    utxoHashes[12],
                    utxoHashes[15]
                ]
            ),
            "Invalid recent roots"
        );

        uint minValidators = (validatorSet.validatorsArray.length * 2) / 3 + 1;
        require(
            signatures.length >= minValidators,
            "Not enough signatures from validators to verify block"
        );

        bytes32 proposalHash = keccak256(
            abi.encode(newRoot, height, otherHashFromBlockHash)
        );
        bytes32 acceptMsg = keccak256(abi.encode(height + 1, proposalHash));
        bytes32 sigMsg = keccak256(
            abi.encodePacked(NETWORK_LEN, NETWORK, acceptMsg)
        );

        require(signatures.length > 0, "No signatures");
        address previous = address(0);
        for (uint i = 0; i < signatures.length; i++) {
            Signature calldata signature = signatures[i];
            address signer = ecrecover(
                sigMsg,
                uint8(signature.v),
                signature.r,
                signature.s
            );
            require(
                validatorSet.validators[signer] == true,
                "Signer is not a validator"
            );

            require(signer > previous, "Signers are not sorted");
            previous = signer;
        }

        aggregateVerifier.verify(
            aggrProof,
            aggrInstances,
            oldRoot,
            newRoot,
            utxoHashes
        );

        addRootHash(newRoot);
        blockHash = proposalHash;

        blockHeight = height;
    }
}
