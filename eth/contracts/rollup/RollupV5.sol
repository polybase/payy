// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "./RollupV4.sol";
import "../BurnVerifierV2.sol";

struct BurnToRouter {
    address router;
    bytes routerCalldata;
    uint256 amount;
    // if the router call fails, return the funds to this address
    address returnAddress;
}

contract RollupV5 is RollupV4 {
    event BlockVerified(uint256 indexed height, bytes32 root);
    event BurnedToAddress(address indexed to, uint256 value);
    event BurnedToRouter(address indexed router, uint256 value);
    event Minted(bytes32 indexed commitment, uint256 value);

    mapping(bytes32 => bytes32) burnsKind;
    mapping(bytes32 => BurnToRouter) burnsToRouter;
    mapping(address => bool) routerWhitelist;

    BurnVerifierV2 public burnVerifierV2;

    function initializeV5(
        address _burnVerifierV2
    ) public reinitializer(5) {
        version = 5;
        burnVerifierV2 = BurnVerifierV2(_burnVerifierV2);
    }

    function DOMAIN_SEPARATOR() public view returns (bytes32) {
        return
            keccak256(
                abi.encode(
                    keccak256(
                        "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
                    ),
                    keccak256(bytes("Rollup")),
                    keccak256(bytes("1")),
                    block.chainid,
                    address(this)
                )
            );
    }

    function isUSDCBlacklisted(address account) internal view returns (bool) {
        return usdc.isBlacklisted(account);
    }

    function requireNotUSDCBlacklisted(address account) internal view {
        require(!isUSDCBlacklisted(account), "account is blacklisted by USDC");
    }

    function addRouter(address router) public onlyOwner {
        require(router.code.length > 0, "Router has no code");
        routerWhitelist[router] = true;
    }

    function removeRouter(address router) public onlyOwner {
        routerWhitelist[router] = false;
    }

    function isRouterWhitelisted(address router) public view returns (bool) {
        return routerWhitelist[router];
    }

    function hasBurn(bytes32 nullifier) public view returns (bool) {
        if (burnsKind[nullifier] == 0) {
            return burns[nullifier].amount != 0;
        } else {
            // if burnsKind is not zero, then there exists a burn
            return true;
        }
    }

    // Anyone can call mint, although this is likely to be performed on behalf of the user
    // as they may not have gas to pay for the txn
    function mint(
        bytes calldata proof,
        bytes32 commitment,
        bytes32 value,
        bytes32 source
    ) public override virtual {
        if (mints[commitment] != 0) {
            revert("Mint already exists");
        }

        mintVerifier.verify(proof, [commitment, value, source]);

        // Take the money from the external account, sender must have been previously
        // approved as per the ERC20 standard
        require(
            IERC20(usdc).transferFrom(msg.sender, address(this), uint256(value)),
            "Transfer failed"
        );

        // Add mint to pending mints, this still needs to be verifier with the verifyBlock,
        // but Solid validators will check that this commitment exists in the mint map before
        // accepting the mint txn into a block
        mints[commitment] = uint256(value);
    }

    function mintWithAuthorization(
        bytes calldata proof,
        bytes32 commitment,
        bytes32 value,
        bytes32 source,
        address from,
        uint256 validAfter,
        uint256 validBefore,
        bytes32 nonce,
        uint256 v,
        bytes32 r,
        bytes32 s,
        // Second signature, not for receiveWithAuthorization,
        // but for this mintWithAuthorization call
        uint256 v2,
        bytes32 r2,
        bytes32 s2
    ) public override virtual {
        if (mints[commitment] != 0) {
            revert("Mint already exists");
        }

        bytes32 structHash = keccak256(
            abi.encode(
                MINT_WITH_AUTHORIZATION_TYPE_HASH,
                commitment,
                value,
                source,
                from,
                validAfter,
                validBefore,
                nonce
            )
        );
        bytes32 computedHash = keccak256(
            abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR(), structHash)
        );
        address signer = ECDSA.recover(computedHash, uint8(v2), r2, s2);
        require(signer == from, "Invalid signer");

        mintVerifier.verify(proof, [commitment, value, source]);

        IUSDC(usdc).receiveWithAuthorization(
            from,
            address(this),
            uint256(value),
            validAfter,
            validBefore,
            nonce,
            uint8(v),
            r,
            s
        );

        mints[commitment] = uint256(value);
    }

    // Anyone can call burn, although this is likely to be performed on behalf of the user
    // as they may not have gas to pay for the txn
    function burn(
        // to address is not verified, we don't care who they send it to
        address to,
        bytes calldata proof,
        bytes32 nullifer,
        bytes32 value,
        bytes32 source,
        bytes32 sig
    ) public override virtual {
        requireNotUSDCBlacklisted(to);

        burnVerifier.verify(
            proof,
            [bytes32(uint256(uint160(to))), nullifer, value, source, sig]
        );

        // Add burn to pending burns, this still needs to be verifier with the verifyBlock,
        // but validators will check that this nullifier exists in the burn map before
        // accepting the burn txn into a block
        burns[nullifer] = Burn(to, uint256(value));
    }

    function burnToAddress(
        bytes32 kind,
        bytes32 to,
        bytes calldata proof,
        bytes32 nullifier,
        bytes32 value,
        bytes32 source,
        bytes32 sig
    ) public virtual {
        require(kind == bytes32(0), "Invalid kind");

        address toAddr = bytes32ToAddress(to);
        requireNotUSDCBlacklisted(toAddr);

        burnVerifierV2.verify(proof, [kind, to, nullifier, value, source, sig]);

        // Add burn to pending burns, burn will be processed when verifyBlock is called
        burnsKind[nullifier] = kind;
        burns[nullifier] = Burn(toAddr, uint256(value));
    }

    function burnToRouter(
        bytes32 kind,
        bytes32 msgHash,
        bytes calldata proof,
        bytes32 nullifier,
        bytes32 value,
        bytes32 source,
        bytes32 sig,
        address router,
        // TODO: this could be big and use a lot of gas to set in storage, set a limit in guild
        bytes calldata routerCalldata,
        address returnAddress
    ) public virtual {
        require(kind == bytes32(uint256(1)), "Invalid kind");
        require(returnAddress != address(0), "Invalid return address");
        require(isRouterWhitelisted(router), "Router not whitelisted");

        bytes32 computedMsg = keccak256(abi.encode(router, routerCalldata, returnAddress));
        // Clear the first 3 bits, BN256 can't fit the full 256 bits
        computedMsg &= bytes32(
            uint256(
                0x1FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
            )
        );
        require(computedMsg == msgHash, "Invalid msg");

        burnVerifierV2.verify(
            proof,
            [kind, msgHash, nullifier, value, source, sig]
        );

        burnsKind[nullifier] = kind;
        burnsToRouter[nullifier] = BurnToRouter(
            router,
            routerCalldata,
            uint256(value),
            returnAddress
        );
    }

    // Verify a new block
    function verifyBlock(
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
        verifyBlock2(
            aggrProof,
            aggrInstances,
            oldRoot,
            newRoot,
            utxoHashes,
            otherHashFromBlockHash,
            height,
            signatures,
            500_000
        );
    }

    // Verify a new block
    // TODO: I added '2' at the end of this function name,
    // because web3 Rust API takes the first verifyBlock function it finds,
    // and it was taking the old one and failing to encode parameters
    function verifyBlock2(
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
        Signature[] calldata signatures,
        uint256 gasPerBurnCall
    ) public onlyProver {
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

            verifyTxn(mb, value, gasPerBurnCall);
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
            address signer = ECDSA.recover(
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

        emit BlockVerified(height, newRoot);
    }

    function verifyTxn(bytes32 mb, bytes32 value, uint256 gasPerBurnCall) internal virtual {
        if (value == 0) {
            return;
        }

        if (mints[mb] != 0) {
            require(mints[mb] == uint256(value), "Invalid mint amount");
            emit Minted(mb, mints[mb]);
            delete mints[mb];
            return;
        }

        if (burnsKind[mb] == 0) {
            // Regular burn to address or no burn at all
            if (burns[mb].amount != 0) {
                require(
                    burns[mb].amount == uint256(value),
                    "Invalid burn amount"
                );

                // You cannot transfer to the zero address,
                // otherwise you get 'ERC20: transfer to the zero address'
                if (burns[mb].to != address(0)) {
                    if (!isUSDCBlacklisted(burns[mb].to)) {
                        // Perform the transfer to the requested account
                        IERC20(usdc).transfer(
                            burns[mb].to,
                            burns[mb].amount
                        );
                    }
                }

                emit BurnedToAddress(burns[mb].to, burns[mb].amount);
                delete burns[mb];
                return;
            } else {
                // Burn doesn't exist, do nothing
            }
        } else if (burnsKind[mb] == bytes32(uint256(1))) {
            // Burn to router
            BurnToRouter memory b = burnsToRouter[mb];
            require(b.amount == uint256(value), "Invalid burn amount");

            IERC20(usdc).approve(b.router, b.amount);
            if (!_routerCall(b.router, b.routerCalldata, gasPerBurnCall)) {
                // Reset allowance to 0
                // TODO: if we reset to 1, we could save gas on future approves?
                IERC20(usdc).approve(b.router, 0);
                // Return the funds to the return address
                IERC20(usdc).transfer(b.returnAddress, b.amount);
            }

            emit BurnedToRouter(b.router, b.amount);
            delete burnsKind[mb];
            return;
        }

        // TODO: currently existance of burn must be checked by validators, because if a nullifier is
        // is not present, the block production will stall here. Given its the users funds
        // at risk, only the user should be responsible for this. In the case a nullifier is missing for
        // a burn, we should just burn the funds and continue. To do this, we need to differentiate between
        // mints and burns, as mints will still need to be checked.
        revert("Invalid mint/burn");
    }

    function _routerCall(address router, bytes memory routerCalldata, uint256 gasPerBurnCall) internal returns (bool) {
        if (!isRouterWhitelisted(router)) {
            return false;
        }

        (bool success, /*bytes memory data*/) = router.call{
            value: 0,
            // TODO: how much gas should we allow
            gas: gasPerBurnCall
        }(routerCalldata);

        return success;
    }

    function bytes32ToAddress(bytes32 _bytes32) public pure returns (address) {
        return address(uint160(uint256(_bytes32)));
    }
}
