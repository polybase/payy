// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import "../AggregateVerifierV1.sol";
import "../MintVerifierV1.sol";
import "../BurnVerifierV1.sol";
import "../IUSDC.sol";
// import "hardhat/console.sol";

struct Block {
    // Sequencer/prover sending the block
    address sender;
    // List of txns in the block
    Transaction[] txns;
}

struct Transaction {
    bytes[] hashes;
}

struct Proof {
    bytes[] proof;
}

struct Signature {
    bytes32 r;
    bytes32 s;
    uint v;
}

struct Mint {
    uint256 amount;
}

struct Burn {
    address to;
    uint256 amount;
}

struct ValidatorSet {
    mapping(address => bool) validators;
    address[] validatorsArray;
    // The height at which this validator set becomes valid, inclusive
    uint256 validFrom;
}

struct PublicValidatorSet {
    // We can't return a mapping from a public function, so we need an array
    address[] validators;
    uint256 validFrom;
}

string constant NETWORK = "Polybase";
uint64 constant NETWORK_LEN = 8;

contract RollupV1 is Initializable, OwnableUpgradeable {
    event ValidatorSetAdded(uint256 index, uint256 validFrom);

    // Since the Initializable._initialized version number is private, we need to keep track of it ourselves
    uint8 public version;

    bytes32 private _DOMAIN_SEPARATOR;

    AggregateVerifierV1 public aggregateVerifier;
    MintVerifierV1 public mintVerifier;
    BurnVerifierV1 public burnVerifier;
    IUSDC public usdc;

    // Core rollup values
    bytes32 public blockHash;
    uint256 public blockHeight;

    bytes32[64] public rootHashes;
    uint public nextRootHashIndex;

    // Actors
    // mapping(address => uint) sequencers;
    mapping(address => uint) provers;
    mapping(bytes32 => uint256) mints;
    mapping(bytes32 => Burn) burns;

    mapping(uint256 => ValidatorSet) private validatorSets;
    uint256 private validatorSetsLength;
    uint256 private validatorSetIndex;

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(
        address owner,
        address _usdcAddress,
        address _aggregateVerifier,
        address _mintVerifier,
        address _burnVerifier,
        address prover,
        address[] calldata initialValidators,
        bytes32 emptyMerkleTreeRootHash
    ) public initializer {
        version = 1;

        __Ownable_init(owner);

        _DOMAIN_SEPARATOR = keccak256(
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

        usdc = IUSDC(_usdcAddress);
        aggregateVerifier = AggregateVerifierV1(_aggregateVerifier);
        mintVerifier = MintVerifierV1(_mintVerifier);
        burnVerifier = BurnVerifierV1(_burnVerifier);
        provers[prover] = 1;

        _setValidators(0, initialValidators);

        addRootHash(emptyMerkleTreeRootHash);
    }

    modifier onlyProver() {
        require(provers[msg.sender] == 1, "You are not a prover");
        _;
    }

    function addRootHash(bytes32 rootHash) internal {
        rootHashes[nextRootHashIndex] = rootHash;
        nextRootHashIndex = (nextRootHashIndex + 1) % 64;
    }

    function currentRootHash() public view returns (bytes32) {
        uint index = 63;
        if (nextRootHashIndex > 0) {
            index = nextRootHashIndex - 1;
        }

        return rootHashes[index];
    }

    function containsRootHashes(
        bytes32[6] memory hashes
    ) public view virtual returns (bool) {
        bool[6] memory results = [false, false, false, false, false, false];

        for (uint i = 0; i < hashes.length; i++) {
            for (uint j = 0; j < rootHashes.length; j++) {
                if (hashes[i] == rootHashes[j]) {
                    results[i] = true;
                    break;
                }
            }
        }

        for (uint i = 0; i < results.length; i++) {
            if (results[i] == false) {
                return false;
            }
        }

        return true;
    }

    function getMint(bytes32 key) public view returns (uint256) {
        return mints[key];
    }

    function getBurn(bytes32 key) public view returns (Burn memory) {
        return burns[key];
    }

    function getRootHashes() public view returns (bytes32[64] memory) {
        return rootHashes;
    }

    function addProver(address prover) public onlyOwner {
        provers[prover] = 1;
    }

    // Returns all validator sets from a given index, inclusive
    function getValidatorSets(
        uint256 from
    ) public view returns (PublicValidatorSet[] memory) {
        PublicValidatorSet[] memory sets = new PublicValidatorSet[](
            validatorSetsLength - from
        );

        for (uint256 i = from; i < validatorSetsLength; i++) {
            sets[i - from] = PublicValidatorSet(
                validatorSets[i].validatorsArray,
                validatorSets[i].validFrom
            );
        }

        return sets;
    }

    function getValidators() internal view returns (ValidatorSet storage) {
        return validatorSets[validatorSetIndex];
    }

    function _setValidators(
        uint256 validFrom,
        address[] calldata validators
    ) private {
        require(
            validatorSetsLength == 0 ||
                validatorSets[validatorSetsLength - 1].validFrom < validFrom,
            "New validator set must have a validFrom greater than the last set"
        );

        validatorSets[validatorSetsLength].validFrom = validFrom;
        validatorSets[validatorSetsLength].validatorsArray = validators;

        for (uint256 i = 0; i < validators.length; i++) {
            require(
                validatorSets[validatorSetsLength].validators[validators[i]] ==
                    false,
                "Validator already exists"
            );

            validatorSets[validatorSetsLength].validators[validators[i]] = true;
        }

        emit ValidatorSetAdded(validatorSetsLength, validFrom);
        validatorSetsLength += 1;
    }

    function setValidators(
        uint256 validFrom,
        address[] calldata validators
    ) public onlyOwner {
        _setValidators(validFrom, validators);
    }

    function updateValidatorSetIndex(uint256 height) internal {
        for (uint256 i = validatorSetIndex + 1; i < validatorSetsLength; i++) {
            if (validatorSets[i].validFrom > height) {
                break;
            }

            validatorSetIndex = i;
        }
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
    ) public virtual onlyProver {
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

                // Perform the transfer to the requested account
                require(
                    usdc.transfer(burns[mb].to, burns[mb].amount),
                    "Transfer failed"
                );

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

    // Anyone can call mint, although this is likely to be performed on behalf of the user
    // as they may not have gas to pay for the txn
    function mint(
        bytes calldata proof,
        bytes32 commitment,
        bytes32 value,
        bytes32 source
    ) public virtual {
        if (mints[commitment] != 0) {
            revert("Mint already exists");
        }

        mintVerifier.verify(proof, [commitment, value, source]);

        // Take the money from the external account, sender must have been previously
        // approved as per the ERC20 standard
        require(
            usdc.transferFrom(msg.sender, address(this), uint256(value)),
            "Transfer failed"
        );

        // Add mint to pending mints, this still needs to be verifier with the verifyBlock,
        // but Solid validators will check that this commitment exists in the mint map before
        // accepting the mint txn into a block
        mints[commitment] = uint256(value);
    }

    bytes32 constant MINT_WITH_AUTHORIZATION_TYPE_HASH =
        keccak256(
            "MintWithAuthorization(bytes32 commitment,bytes32 value,bytes32 source,address from,uint256 validAfter,uint256 validBefore,bytes32 nonce)"
        );

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
    ) public virtual {
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
            abi.encodePacked("\x19\x01", _DOMAIN_SEPARATOR, structHash)
        );
        address signer = ecrecover(computedHash, uint8(v2), r2, s2);
        require(signer == from, "Invalid signer");

        mintVerifier.verify(proof, [commitment, value, source]);

        usdc.receiveWithAuthorization(
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
    ) public virtual {
        burnVerifier.verify(
            proof,
            [bytes32(uint256(uint160(to))), nullifer, value, source, sig]
        );

        // Add burn to pending burns, this still needs to be verifier with the verifyBlock,
        // but validators will check that this nullifier exists in the burn map before
        // accepting the burn txn into a block
        burns[nullifer] = Burn(to, uint256(value));
    }

    function setRoot(bytes32 newRoot) public onlyOwner {
        addRootHash(newRoot);
    }
}
