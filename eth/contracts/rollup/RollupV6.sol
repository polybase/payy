// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "./RollupV5.sol";

contract RollupV6 is RollupV5 {
    event Burned(bytes32 indexed nullifier, bool substitute, bool success);
    event BurnAdded(bytes32 indexed nullifier, uint256 amount);
    event MintAdded(bytes32 indexed commitment, uint256 amount);

    mapping(bytes32 => bool) public substitutedBurns;
    mapping(bytes32 => bool) public rolledUpLeafs;

    uint256 public gasPerRouterCall;

    function initializeV6() public reinitializer(6) {
        version = 6;

        gasPerRouterCall = 500_000;
    }

    function setGasPerRouterCall(uint256 _gasPerRouterCall) public onlyOwner {
        gasPerRouterCall = _gasPerRouterCall;
    }

    function setBurnsKind(bytes32 nullifier, bytes32 kind) internal {
        require(
            burnsKind[nullifier] == 0 && burns[nullifier].amount == 0,
            "Burn already exists"
        );
        burnsKind[nullifier] = kind;
    }

    // Anyone can call mint, although this is likely to be performed on behalf of the user
    // as they may not have gas to pay for the txn
    function mint(
        bytes calldata proof,
        bytes32 commitment,
        bytes32 value,
        bytes32 source
    ) public override {
        if (mints[commitment] != 0) {
            revert("Mint already exists");
        }

        mintVerifier.verify(proof, [commitment, value, source]);

        // Take the money from the external account, sender must have been previously
        // approved as per the ERC20 standard
        IERC20(usdc).transferFrom(msg.sender, address(this), uint256(value));

        // Add mint to pending mints, this still needs to be verifier with the verifyBlock,
        // but Solid validators will check that this commitment exists in the mint map before
        // accepting the mint txn into a block
        mints[commitment] = uint256(value);
        emit MintAdded(commitment, uint256(value));
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
    ) public override {
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
        emit MintAdded(commitment, uint256(value));
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
    ) public override {
        requireNotUSDCBlacklisted(to);

        burnVerifier.verify(
            proof,
            [bytes32(uint256(uint160(to))), nullifer, value, source, sig]
        );

        // Add burn to pending burns, this still needs to be verifier with the verifyBlock,
        // but validators will check that this nullifier exists in the burn map before
        // accepting the burn txn into a block
        setBurnsKind(nullifer, 0);
        burns[nullifer] = Burn(to, uint256(value));

        emit BurnAdded(nullifer, uint256(value));
    }

    function burnToAddress(
        bytes32 kind,
        bytes32 to,
        bytes calldata proof,
        bytes32 nullifier,
        bytes32 value,
        bytes32 source,
        bytes32 sig
    ) public override {
        require(kind == bytes32(0), "Invalid kind");

        address toAddr = bytes32ToAddress(to);
        requireNotUSDCBlacklisted(toAddr);

        burnVerifierV2.verify(proof, [kind, to, nullifier, value, source, sig]);

        // Add burn to pending burns, burn will be processed when verifyBlock is called
        setBurnsKind(nullifier, kind);
        burns[nullifier] = Burn(toAddr, uint256(value));
        emit BurnAdded(nullifier, uint256(value));
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
    ) public override {
        require(kind == bytes32(uint256(1)), "Invalid kind");
        require(returnAddress != address(0), "Invalid return address");
        require(isRouterWhitelisted(router), "Router not whitelisted");
        requireNotUSDCBlacklisted(returnAddress);

        bytes32 computedMsg = keccak256(
            abi.encode(router, routerCalldata, returnAddress)
        );
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

        setBurnsKind(nullifier, kind);
        burnsToRouter[nullifier] = BurnToRouter(
            router,
            routerCalldata,
            uint256(value),
            returnAddress
        );
        emit BurnAdded(nullifier, uint256(value));
    }

    function wasBurnSubstituted(bytes32 nullifier) public view returns (bool) {
        return substitutedBurns[nullifier];
    }

    function executeBurnToAddress(
        bytes32 nullifier,
        uint256 value
    ) internal returns (bool) {
        Burn memory b = burns[nullifier];
        address to = b.to;

        if (b.amount == 0) {
            // Burn does not exist
            return false;
        }

        require(b.amount == value, "RollupV6: Invalid burn amount");

        try IERC20(usdc).transfer(to, value) {
            return true;
        } catch {
            return false;
        }
    }

    function executeBurnToRouter(
        bytes32 nullifier,
        uint256 value
    ) internal returns (bool) {
        BurnToRouter memory b = burnsToRouter[nullifier];

        // This should never happen
        require(b.amount == value, "RollupV6: Invalid burn amount");

        IERC20(usdc).approve(b.router, value);
        bool routerCallSuccess = _routerCall(
            b.router,
            b.routerCalldata,
            gasPerRouterCall
        );
        // Reset allowance to 0, even if successful,
        // so there is no residual allowance
        IERC20(usdc).approve(b.router, 0);
        if (!routerCallSuccess) {
            // Call reverted.

            if (isUSDCBlacklisted(b.returnAddress)) {
                return false;
            }

            // Return the funds to the return address
            IERC20(usdc).transfer(b.returnAddress, b.amount);

            // This is still a success, so we don't return false
        }

        return true;
    }

    // First bool is if the burn was found at all,
    // second bool is if the burn was successful
    function executeBurn(
        bytes32 mb,
        uint256 value,
        bool substitute
    ) internal returns (bool, bool) {
        bool success;
        bool found = true;
        if (burnsKind[mb] == 0) {
            if (burns[mb].amount == 0) {
                // Burn does not exist
                found = false;
            }

            success = found && executeBurnToAddress(mb, value);
        } else if (burnsKind[mb] == bytes32(uint256(1))) {
            success = executeBurnToRouter(mb, value);
        } else {
            success = false;
        }

        emit Burned(mb, substitute, success);

        return (found, success);
    }

    function verifyTxn(
        bytes32 mb,
        bytes32 value,
        // TODO: remove this parameter in the next verifyBlock version
        uint256 /*_gasPerRouterCall*/
    ) internal override {
        if (value == 0) {
            return;
        }

        rolledUpLeafs[mb] = true;

        if (mints[mb] != 0) {
            require(
                mints[mb] == uint256(value),
                "RollupV6: Invalid mint amount"
            );
            emit Minted(mb, mints[mb]);
            return;
        }

        // TODO: currently existance of burn must be checked by validators, because if a nullifier is
        // is not present, the block production will stall here. Given its the users funds
        // at risk, only the user should be responsible for this. In the case a nullifier is missing for
        // a burn, we should just burn the funds and continue. To do this, we need to differentiate between
        // mints and burns, as mints will still need to be checked.
        // For now, we revert if a burn is not found.
        (bool found, ) = executeBurn(mb, uint256(value), false);
        require(found, "RollupV6: Burn was not found");
    }

    function substituteBurn(bytes32 nullifier, uint256 amount) public {
        require(
            !substitutedBurns[nullifier],
            "RollupV6: Burn already substituted"
        );
        require(!rolledUpLeafs[nullifier], "RollupV6: Leaf already rolled up");
        IERC20(usdc).transferFrom(msg.sender, address(this), amount);

        substitutedBurns[nullifier] = true;

        (bool found, bool success) = executeBurn(nullifier, amount, true);
        require(found && success, "RollupV6: Burn failed");

        // This will be returned to the msg.sender when the rollup block for it is submitted
        burnsKind[nullifier] = 0;
        burns[nullifier] = Burn({to: msg.sender, amount: amount});
    }
}
