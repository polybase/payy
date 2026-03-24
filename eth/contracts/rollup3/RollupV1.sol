// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IVerifier} from "../noir/IVerifier.sol";
import "../IUSDC.sol";

struct Mint {
    bytes32 note_kind;
    uint256 amount;
    bool spent;
}

struct Signature {
    bytes32 r;
    bytes32 s;
    uint v;
}

struct ValidatorSet {
    mapping(address => bool) validators;
    address[] validatorsArray;
    // The height at which this validator set becomes valid, inclusive
    uint256 validFrom;
}

// We can't return a mapping from a public function, so this struct is used for the public
// return valjue
struct PublicValidatorSet {
    address[] validators;
    uint256 validFrom;
}

// Verifiers
struct Verifier {
    IVerifier verifier;
    uint32 messages_length;
    bool enabled;
}

string constant NETWORK = "Payy";
uint64 constant NETWORK_LEN = 4;
uint256 constant MAX_FUTURE_BLOCKS = 2_592_000; // 30 days (~1 sec blocks)

contract RollupV1 is Initializable, OwnableUpgradeable {
    using SafeERC20 for IERC20;
    event RollupVerified(uint256 indexed height, bytes32 root);
    event Minted(bytes32 indexed hash, bytes32 value, bytes32 note_kind);
    event ValidatorSetAdded(uint256 index, uint256 validFrom);
    event Burned(
        address indexed token,
        bytes32 indexed burn_hash,
        address indexed recipient,
        bool substitute,
        bool success
    );
    event MintAdded(
        bytes32 indexed mint_hash,
        uint256 value,
        bytes32 note_kind
    );
    event VerifierAdded(
        bytes32 verificationKey,
        address zkVerifierAddress,
        uint32 messages_length
    );
    event VerifierRemoved(bytes32 verificationKey, address zkVerifierAddress);
    event ProverAdded(address indexed prover);
    event ProverRemoved(address indexed prover);
    event RootHashUpdated(bytes32 indexed oldRoot, bytes32 indexed newRoot);

    // Contract version tracking for off-chain consumers (merged V2 behavior)
    uint8 public version;

    // Verifiers
    mapping(bytes32 => Verifier) public zkVerifiers;
    bytes32[] public zkVerifierKeys;

    // Contracts
    IUSDC public usdc;

    // Core rollup values
    uint256 public blockHeight;
    bytes32 public rootHash;

    // Mint - mints are removed after the rollup validates them. Mint hash is hash of commitments.
    mapping(bytes32 => Mint) public mints;

    // Burn Substitutor - stores a mapping of paid out substituted burns, so they can be refunded
    // once the rollup completes the original burn
    // Composite key (hash + burnAddress + noteKind + amount) => substitute address
    mapping(bytes32 => address) public substitutedBurns;

    // Allowed Tokens
    mapping(bytes32 => address) tokens;

    // Actors
    mapping(address => uint) provers;

    // Validators
    ValidatorSet[] private validatorSets;
    uint256 private validatorSetIndex;

    // Burn substitutors
    mapping(address => bool) private burnSubstitutors;

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(
        address owner,
        address _usdcAddress,
        address _verifierAddress,
        address prover,
        address[] calldata initialValidators,
        bytes32 verifierKeyHash,
        uint32 verifierMessagesCount,
        bytes32 initialNoteKind
    ) public initializer {
        version = 2;

        __Ownable_init(owner);

        usdc = IUSDC(_usdcAddress);

        // Set the init aggregate verifier
        _setZkVerifierProperties(
            verifierKeyHash,
            _verifierAddress,
            verifierMessagesCount
        );
        zkVerifierKeys.push(verifierKeyHash);

        provers[prover] = 1;

        _setValidators(0, initialValidators);

        // Empty merkle tree root hash constant (from pkg/contracts/src/empty_merkle_tree_root_hash.txt)
        _setRootHash(
            0x0577b5b4aa3eaba75b2a919d5d7c63b7258aa507d38e346bf2ff1d48790379ff
        );
        tokens[initialNoteKind] = _usdcAddress;
        burnSubstitutors[owner] = true;
    }

    modifier onlyProver() {
        require(provers[msg.sender] == 1, "You are not a prover");
        _;
    }

    function addProver(address prover) public onlyOwner {
        provers[prover] = 1;
        emit ProverAdded(prover);
    }

    function removeProver(address prover) public onlyOwner {
        require(provers[prover] == 1, "Address is not a prover");
        provers[prover] = 0;
        emit ProverRemoved(prover);
    }

    modifier onlyBurnSubstitutor() {
        require(
            burnSubstitutors[msg.sender] == true,
            "RollupV1: You are not a burn substitutor"
        );
        _;
    }

    function _setZkVerifierProperties(
        bytes32 keyHash,
        address verifierAddress,
        uint32 messagesLength
    ) internal {
        zkVerifiers[keyHash].verifier = IVerifier(verifierAddress);
        zkVerifiers[keyHash].messages_length = messagesLength;
        zkVerifiers[keyHash].enabled = true;
    }

    function addZkVerifier(
        bytes32 verificationKeyHash,
        address verifierAddress,
        uint32 messages_length
    ) public onlyOwner {
        require(
            verifierAddress != address(0),
            "RollupV1: Invalid zk verifier address"
        );
        require(
            verifierAddress.code.length > 0,
            "RollupV1: ZK verifier is not a contract"
        );
        // Add to verifier keys if not enabled
        if (!zkVerifiers[verificationKeyHash].enabled) {
            zkVerifierKeys.push(verificationKeyHash);
        }
        _setZkVerifierProperties(
            verificationKeyHash,
            verifierAddress,
            messages_length
        );
        emit VerifierAdded(
            verificationKeyHash,
            verifierAddress,
            messages_length
        );
    }

    function removeZkVerifier(bytes32 verificationKeyHash) public onlyOwner {
        require(
            zkVerifiers[verificationKeyHash].enabled,
            "RollupV1: ZK verifier does not exist"
        );
        address verifierAddress = address(
            zkVerifiers[verificationKeyHash].verifier
        );
        delete zkVerifiers[verificationKeyHash];
        // Find and remove the verifierKey from verifierKeys array
        uint256 length = zkVerifierKeys.length;
        for (uint256 i = 0; i < length; i++) {
            if (zkVerifierKeys[i] == verificationKeyHash) {
                // Move the last element to this position and remove the last element
                zkVerifierKeys[i] = zkVerifierKeys[length - 1];
                zkVerifierKeys.pop();
                break;
            }
        }
        emit VerifierRemoved(verificationKeyHash, verifierAddress);
    }

    function getZkVerifier(
        bytes32 verificationKeyHash
    ) public view returns (address, uint32) {
        require(
            zkVerifiers[verificationKeyHash].enabled,
            "RollupV1: ZK verifier does not exist"
        );
        return (
            address(zkVerifiers[verificationKeyHash].verifier),
            zkVerifiers[verificationKeyHash].messages_length
        );
    }

    function addBurnSubstitutor(address burnSubstitutor) public onlyOwner {
        burnSubstitutors[burnSubstitutor] = true;
    }

    function removeBurnSubstitutor(address burnSubstitutor) public onlyOwner {
        burnSubstitutors[burnSubstitutor] = false;
    }

    function _setRootHash(bytes32 newRoot) internal {
        bytes32 oldRoot = rootHash;
        rootHash = newRoot;
        emit RootHashUpdated(oldRoot, newRoot);
    }

    function setRoot(bytes32 newRoot) public onlyOwner {
        _setRootHash(newRoot);
    }

    function currentRootHash() public view returns (bytes32) {
        return rootHash;
    }

    function addToken(bytes32 noteKind, address tokenAddress) public onlyOwner {
        require(
            tokens[noteKind] == address(0),
            "RollupV1: Token already exists"
        );

        tokens[noteKind] = tokenAddress;
    }

    function noteKindTokenAddress(
        bytes32 noteKind
    ) public view returns (address) {
        return tokens[noteKind];
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

    bytes32 constant MINT_WITH_AUTHORIZATION_TYPE_HASH =
        keccak256(
            "MintWithAuthorization(bytes32 commitment,bytes32 value,bytes32 kind,address from,uint256 validAfter,uint256 validBefore,bytes32 nonce)"
        );

    /////////////////
    //
    // VERIFY
    //
    ///////////

    // Verify rollup
    function verifyRollup(
        uint256 height,
        bytes32 verificationKeyHash,
        bytes calldata aggrProof,
        // oldRoot, newRoot, commitHash, <messages_length>, 16x kzg
        bytes32[] calldata publicInputs,
        bytes32 otherHashFromBlockHash,
        Signature[] calldata signatures
    ) public onlyProver {
        require(
            zkVerifiers[verificationKeyHash].enabled,
            "RollupV1: ZK verifier not allowed"
        );

        require(
            publicInputs.length ==
                zkVerifiers[verificationKeyHash].messages_length + 3,
            "RollupV1: Invalid publicInputs length"
        );

        bytes32 oldRoot = publicInputs[0];
        bytes32 newRoot = publicInputs[1];
        bytes32 commitHash = publicInputs[2];

        verifyRootHash(oldRoot);

        verifyCommitHash(commitHash);

        verifyValidatorSignatures(
            newRoot,
            height,
            otherHashFromBlockHash,
            signatures
        );

        verifyAllMessages(
            // Skip the first 3 public inputs as these are never messages
            3,
            publicInputs,
            zkVerifiers[verificationKeyHash].messages_length
        );

        require(
            zkVerifiers[verificationKeyHash].verifier.verify(
                aggrProof,
                publicInputs
            ),
            "RollupV1: ZK proof verification failed"
        );

        rootHash = newRoot;
        require(
            height > blockHeight,
            "RollupV1: New block height must be greater than current"
        );
        blockHeight = height;

        emit RollupVerified(height, newRoot);
    }

    function verifyRootHash(bytes32 expectedRoot) internal view {
        require(
            expectedRoot == rootHash,
            "RollupV1: Root hash verification failed"
        );
    }

    // Placeholder for asserting the commit hash is stored on Celestia
    function verifyCommitHash(bytes32 commitHash) internal {}

    function verifyAllMessages(
        uint skipCount,
        bytes32[] calldata publicInputs,
        uint32 messages_length
    ) internal {
        // Skip the first 4 (as they are processed separately)
        uint i = skipCount;
        uint end = skipCount + messages_length;
        while (i < end) {
            i = verifyMessages(i, publicInputs);
        }
    }

    function verifyMessages(
        uint index,
        bytes32[] calldata publicInputs
    ) internal returns (uint) {
        // Get the kind from public input at index
        uint256 kind = uint256(publicInputs[index]);

        if (kind == 0) {
            return index + 1;
        } else if (kind == 2) {
            // Mint
            return verifyMint(index, publicInputs);
        } else if (kind == 3) {
            // Burn
            return verifyBurn(index, publicInputs);
        } else {
            // Not allowed
            revert("RollupV1: Invalid message kind");
        }
    }

    function verifyMint(
        uint i,
        bytes32[] calldata messages
    ) internal returns (uint) {
        bytes32 note_kind = messages[i + 1];
        bytes32 value = messages[i + 2];
        bytes32 hash = messages[i + 3];

        require(
            mints[hash].amount == uint256(value),
            "RollupV1: Mint value invalid"
        );
        require(
            mints[hash].note_kind == note_kind,
            "RollupV1: Mint note kind invalid"
        );
        require(mints[hash].spent == false, "RollupV1: Mint already spent");
        mints[hash].spent = true;

        emit Minted(hash, value, note_kind);

        return i + 4;
    }

    function verifyBurn(
        uint i,
        bytes32[] calldata messages
    ) internal returns (uint) {
        bytes32 note_kind = messages[i + 1];
        uint256 value = uint256(messages[i + 2]);
        bytes32 hash = messages[i + 3];
        address burn_addr = bytes32ToAddress(messages[i + 4]);

        address token = tokens[note_kind];

        bytes32 substituteBurnKey = getSubstituteBurnKey(
            hash,
            burn_addr,
            note_kind,
            value
        );
        address substitutor = substitutedBurns[substituteBurnKey];
        if (substitutor != address(0)) {
            bool success = executeBurn(token, substitutor, hash, value, false);
            if (success) {
                delete substitutedBurns[substituteBurnKey];
            }
        } else {
            executeBurn(token, burn_addr, hash, value, false);
        }

        return i + 5;
    }

    function bytes32ToAddress(
        bytes32 _bytes32
    ) internal pure returns (address) {
        return address(uint160(uint256(_bytes32)));
    }

    /**
     * @dev Helper function to generate composite key for substitutedBurns mapping
     * @param hash The burn hash
     * @param burnAddress The burn address
     * @param noteKind The note kind
     * @param amount The amount
     * @return The composite key for the mapping
     */
    function getSubstituteBurnKey(
        bytes32 hash,
        address burnAddress,
        bytes32 noteKind,
        uint256 amount
    ) internal pure returns (bytes32) {
        return
            keccak256(
                abi.encode(hash, burnAddress, noteKind, amount, uint256(0))
            );
    }

    /////////////////
    //
    // BURNS
    //
    ///////////

    function executeBurn(
        address token,
        address recipient,
        bytes32 burn_hash,
        uint256 value,
        bool substitute
    ) internal returns (bool) {
        bool success = executeBurnToAddress(token, recipient, value);
        emit Burned(token, burn_hash, recipient, substitute, success);
        return success;
    }

    function executeBurnToAddress(
        address token,
        address recipient,
        uint256 value
    ) internal returns (bool) {
        (bool success, bytes memory returndata) = token.call(
            abi.encodeCall(IERC20.transfer, (recipient, value))
        );
        if (!success) {
            return false;
        }
        if (returndata.length != 0) {
            bool func_return = abi.decode(returndata, (bool));
            if (!func_return) {
                return false;
            }
        }
        return true;
    }

    function wasBurnSubstituted(
        address burn_address,
        bytes32 note_kind,
        bytes32 hash,
        uint256 amount
    ) public view returns (bool) {
        bytes32 substituteBurnKey = getSubstituteBurnKey(
            hash,
            burn_address,
            note_kind,
            amount
        );
        return substitutedBurns[substituteBurnKey] != address(0);
    }

    function substituteBurn(
        address burnAddress,
        bytes32 note_kind,
        bytes32 hash,
        uint256 amount,
        uint256 burnBlockHeight
    ) public onlyBurnSubstitutor {
        substituteBurnTo(
            burnAddress,
            msg.sender,
            note_kind,
            hash,
            amount,
            burnBlockHeight
        );
    }

    function substituteBurnTo(
        address burnAddress,
        address substituteAddress,
        bytes32 note_kind,
        bytes32 hash,
        uint256 amount,
        uint256 burnBlockHeight
    ) private {
        bytes32 substituteBurnKey = getSubstituteBurnKey(
            hash,
            burnAddress,
            note_kind,
            amount
        );
        require(
            substitutedBurns[substituteBurnKey] == address(0),
            "RollupV1: Burn already substituted"
        );
        require(
            blockHeight < burnBlockHeight,
            "RollupV1: Block height already rolled up"
        );

        address token = tokens[note_kind];
        require(token != address(0), "RollupV1: Token not found for note kind");

        IERC20(token).safeTransferFrom(
            substituteAddress,
            address(this),
            amount
        );

        bool success = executeBurn(token, burnAddress, hash, amount, true);
        require(success, "RollupV1: Burn failed");

        // This will be returned to the msg.sender when the rollup block for it is submitted
        substitutedBurns[substituteBurnKey] = substituteAddress;
    }

    /////////////////
    //
    // MINTS
    //
    ///////////

    function getMint(bytes32 hash) public view returns (Mint memory) {
        return mints[hash];
    }

    // Anyone can call mint, although this is likely to be performed on behalf of the user
    // as they may not have gas to pay for the txn
    function mint(bytes32 mint_hash, bytes32 value, bytes32 note_kind) public {
        if (mints[mint_hash].amount != 0) {
            revert("RollupV1: Mint already exists");
        }

        address tokenAddress = tokens[note_kind];
        require(
            tokenAddress != address(0),
            "RollupV1: Token not found for note kind"
        );

        // Take the money from the external account, sender must have been previously
        // approved as per the ERC20 standard
        IERC20(tokenAddress).safeTransferFrom(
            msg.sender,
            address(this),
            uint256(value)
        );

        // Add mint to pending mints, this still needs to be verifier with the verifyBlock,
        // but Solid validators will check that this commitment exists in the mint map before
        // accepting the mint txn into a block
        mints[mint_hash] = Mint({
            note_kind: note_kind,
            amount: uint256(value),
            spent: false
        });

        emit MintAdded(mint_hash, uint256(value), note_kind);
    }

    function mintWithAuthorization(
        bytes32 mint_hash,
        bytes32 value,
        bytes32 note_kind,
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
    ) public {
        if (mints[mint_hash].amount != 0) {
            revert("RollupV1: Mint already exists");
        }

        bytes32 structHash = keccak256(
            abi.encode(
                MINT_WITH_AUTHORIZATION_TYPE_HASH,
                mint_hash,
                value,
                note_kind,
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
        require(signer == from, "RollupV1: Invalid signer");

        address tokenAddress = tokens[note_kind];
        require(
            tokenAddress != address(0),
            "RollupV1: Token not found for note kind"
        );

        // This will fail if the token does not support receiveWithAuthorization
        // method in the defined format. Users of this method must ensure that
        // the token supports it.
        IUSDC(tokenAddress).receiveWithAuthorization(
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

        mints[mint_hash] = Mint({
            note_kind: note_kind,
            amount: uint256(value),
            spent: false
        });
        emit MintAdded(mint_hash, uint256(value), note_kind);
    }



    /////////////////
    //
    // VALIDATORS
    //
    ///////////
    function verifyValidatorSignatures(
        bytes32 newRoot,
        uint256 height,
        bytes32 otherHashFromBlockHash,
        Signature[] calldata signatures
    ) internal {
        updateValidatorSetIndex(height);
        ValidatorSet storage validatorSet = getValidators();

        require(signatures.length > 0, "RollupV1: No signatures");

        uint minValidators = (validatorSet.validatorsArray.length * 2) / 3 + 1;
        require(
            signatures.length >= minValidators,
            "RollupV1: Not enough signatures from validators to verify block"
        );

        bytes32 sigHash = getSignatureMessageHash(
            newRoot,
            height,
            otherHashFromBlockHash
        );

        address previous = address(0);
        for (uint i = 0; i < signatures.length; i++) {
            Signature calldata signature = signatures[i];
            address signer = ECDSA.recover(
                sigHash,
                uint8(signature.v),
                signature.r,
                signature.s
            );
            require(
                validatorSet.validators[signer] == true,
                "RollupV1: Signer is not a validator"
            );

            require(signer > previous, "RollupV1: Signers are not sorted");
            previous = signer;
        }
    }

    function getSignatureMessageHash(
        bytes32 newRoot,
        uint256 height,
        bytes32 otherHashFromBlockHash
    ) internal pure returns (bytes32) {
        bytes32 proposalHash = keccak256(
            abi.encode(newRoot, height, otherHashFromBlockHash)
        );
        bytes32 acceptMsg = keccak256(abi.encode(height + 1, proposalHash));
        bytes32 sigMsg = keccak256(
            abi.encodePacked(NETWORK_LEN, NETWORK, acceptMsg)
        );
        return sigMsg;
    }

    // Returns all validator sets from a given index, inclusive
    function getValidatorSets(
        uint256 from
    ) public view returns (PublicValidatorSet[] memory) {
        PublicValidatorSet[] memory sets = new PublicValidatorSet[](
            validatorSets.length - from
        );

        for (uint256 i = from; i < validatorSets.length; i++) {
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
            validatorSets.length == 0 ||
                validatorSets[validatorSets.length - 1].validFrom < validFrom,
            "RollupV1: New validator set must have a validFrom greater than the last set"
        );
        require(
            validFrom == 0 || validFrom <= block.number + MAX_FUTURE_BLOCKS,
            "RollupV1: validFrom cannot be more than 30 days in the future"
        );

        // Create a new ValidatorSet and push it to the array
        validatorSets.push();
        uint256 newIndex = validatorSets.length - 1;

        validatorSets[newIndex].validFrom = validFrom;
        validatorSets[newIndex].validatorsArray = validators;

        for (uint256 i = 0; i < validators.length; i++) {
            require(
                validatorSets[newIndex].validators[validators[i]] == false,
                "RollupV1: Validator already exists"
            );

            validatorSets[newIndex].validators[validators[i]] = true;
        }

        emit ValidatorSetAdded(newIndex, validFrom);
    }

    function setValidators(
        uint256 validFrom,
        address[] calldata validators
    ) public onlyOwner {
        _setValidators(validFrom, validators);
    }

    function updateValidatorSetIndex(uint256 height) internal {
        for (uint256 i = validatorSetIndex + 1; i < validatorSets.length; i++) {
            if (validatorSets[i].validFrom > height) {
                break;
            }

            validatorSetIndex = i;
        }
    }

    // Debug function to expose internal hash calculation
    function debugGetSignatureMessageHash(
        bytes32 newRoot,
        uint256 height,
        bytes32 otherHashFromBlockHash
    ) external pure returns (bytes32) {
        return getSignatureMessageHash(newRoot, height, otherHashFromBlockHash);
    }

    // Debug function to expose intermediate values
    function debugGetIntermediateHashes(
        bytes32 newRoot,
        uint256 height,
        bytes32 otherHashFromBlockHash
    )
        external
        pure
        returns (bytes32 proposalHash, bytes32 acceptMsg, bytes32 sigMsg)
    {
        proposalHash = keccak256(
            abi.encode(newRoot, height, otherHashFromBlockHash)
        );
        acceptMsg = keccak256(abi.encode(height + 1, proposalHash));
        sigMsg = keccak256(abi.encodePacked(NETWORK_LEN, NETWORK, acceptMsg));
    }

    // Debug function to see the raw packed bytes
    function debugGetPackedBytes(
        bytes32 acceptMsg
    ) external pure returns (bytes memory) {
        return abi.encodePacked(NETWORK_LEN, NETWORK, acceptMsg);
    }
}
