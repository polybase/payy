// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.21;

import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {HonkVerifier as AggAggVerifier} from "../noir/agg_agg.sol";
import {HonkVerifier as MintVerifier} from "../noir/mint.sol";
import "../IUSDC.sol";
import "./base/Util.sol";

// import "hardhat/console.sol";

struct Mint {
    uint256 note_kind;
    uint256 amount;
}

contract RollupV1 is Initializable, OwnableUpgradeable {
    event RollupVerified(uint256 indexed height, bytes32 root);
    event Minted(bytes32 indexed hash, uint256 value, address token);
    event Burned(bytes32 indexed nullifier, bool substitute, bool success);
    event BurnAdded(bytes32 indexed nullifier, uint256 amount);
    event MintAdded(
        bytes32 indexed commitment,
        uint256 amount,
        bytes32 note_kind
    );

    // Since the Initializable._initialized version number is private, we need to keep track of it ourselves
    uint8 public version;

    // Contracts
    MintVerifier public mintVerifier;
    AggAggVerifier public aggregateVerifier;
    IUSDC public usdc;

    // Core rollup values
    // TODO: do we need height?
    uint256 public blockHeight;
    bytes32 public rootHash;

    // Mint/Burns
    mapping(uint160 => Mint) public mints;
    // hash => amount => to address
    mapping(bytes32 => mapping(uint256 => address)) public substitutedBurns;

    // Allowed Tokens
    mapping(uint256 => address) tokens;

    // Actors
    mapping(address => uint) provers;

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

        usdc = IUSDC(_usdcAddress);
        aggregateVerifier = AggAggVerifier(_aggregateVerifier);
        provers[prover] = 1;

        setRoot(emptyMerkleTreeRootHash);
    }

    modifier onlyProver() {
        require(provers[msg.sender] == 1, "You are not a prover");
        _;
    }

    function addProver(address prover) public onlyOwner {
        provers[prover] = 1;
    }

    function setRoot(bytes32 newRoot) public onlyOwner {
        rootHash = newRoot;
    }

    // TODO: add/remove tokens

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
            "MintWithAuthorization(bytes32 commitment,bytes32 value,bytes32 source,address from,uint256 validAfter,uint256 validBefore,bytes32 nonce)"
        );

    /////////////////
    //
    // VERIFY
    //
    ///////////

    // TODO: we should break up this fn for more re-use
    // TODO: we should make messages variable to support different
    // numbers of messages
    // Verify rollup with 36 messages
    function verifyRollup36(
        // Transaction[] calldata txns,
        uint256 height,
        bytes calldata aggrProof,
        // oldRoot, newRoot, 6 utxo x 6 messages per utxo
        bytes32[] calldata publicInputs
    ) public onlyProver {
        bytes32 oldRoot = publicInputs[0];
        bytes32 newRoot = publicInputs[1];

        require(
            oldRoot == rootHash,
            "Old root does not match the current root"
        );

        // Check mints/burns
        uint i = 0;
        while (i < 12) {
            // 2 + to skip oldRoot and newRoot
            i = verifyMessages(2 + i, publicInputs);
        }

        aggregateVerifier.verify(aggrProof, publicInputs);

        setRoot(newRoot);
        rootHash = newRoot;
        blockHeight = height;

        emit RollupVerified(height, newRoot);
    }

    // TODO: maybe messages should be variable length
    function verifyMessages(
        uint index,
        // This is actually publicInputs, which includes messages
        bytes32[] calldata messages
    ) internal returns (uint) {
        // Get the kind from last byte (least sig number)
        // TODO: change this to data[0]
        uint8 kind = uint8(bytes1(messages[index][31]));

        if (kind == 0) {
            return index + 1;
        }

        // Mint
        if (kind == 2) {
            return verifyMint(index, messages);
        }

        // Burn
        if (kind == 3) {
            return verifyBurn(index, messages);
        }

        // Not allowed
        revert("Invalid message kind");
    }

    function verifyMint(
        uint i,
        bytes32[] calldata messages
    ) internal returns (uint) {
        bytes32 note_kind = messages[i + 1];
        bytes32 value = messages[i + 2];
        // TODO: is the cast to uint160 correct?
        uint160 hash = uint160(bytes20(messages[i + 3]));

        require(mints[hash].amount == uint256(value), "Mint value invalid");
        require(
            mints[hash].note_kind == uint256(note_kind),
            "Mint note kind invalid"
        );

        // Remove the mint once we've ack it
        mints[hash].note_kind = 0;
        mints[hash].amount = 0;

        return i + 6;
    }

    function verifyBurn(
        uint i,
        bytes32[] calldata messages
    ) internal returns (uint) {
        bytes32 note_kind = messages[i + 1];
        uint256 value = uint256(messages[i + 2]);
        // TODO: should we combine hash and burn addr? would it cause a vulnerability?
        bytes32 hash = messages[i + 3];
        bytes32 burn_addr = messages[i + 4];

        if (substitutedBurns[hash][value] != address(0)) {
            executeBurn(substitutedBurns[hash][value], hash, value, false);
        } else {
            // TODO: is this cast to address correct?
            executeBurn(address(bytes20(burn_addr)), hash, value, false);
        }

        return i + 6;
    }

    function bytes32ToAddress(bytes32 _bytes32) public pure returns (address) {
        // TODO: can we not do address(uint160(_bytes32))
        return address(uint160(uint256(_bytes32)));
    }

    /////////////////
    //
    // BURNS
    //
    ///////////

    function executeBurn(
        address recipient,
        bytes32 nullifier,
        uint256 value,
        bool substitute
    ) internal returns (bool) {
        bool success = executeBurnToAddress(recipient, value);
        emit Burned(nullifier, substitute, success);
        return success;
    }

    function executeBurnToAddress(
        address recipient,
        uint256 value
    ) internal returns (bool) {
        try IERC20(usdc).transfer(recipient, value) {
            return true;
        } catch {
            return false;
        }
    }

    function wasBurnSubstituted(
        bytes32 hash,
        uint256 amount
    ) public view returns (bool) {
        // TODO
        assert(false);
        // return substitutedBurns[hash][amount] != 0;
    }

    function substituteBurn(
        bytes32 hash,
        uint256 amount,
        uint256 burnBlockHeight
    ) public {
        substituteBurnTo(msg.sender, hash, amount, burnBlockHeight);
    }

    function substituteBurnTo(
        address to,
        bytes32 hash,
        uint256 amount,
        uint256 burnBlockHeight
    ) public {
        // TODO
        assert(false);
        // require(
        //     !substitutedBurns[hash][amount],
        //     "RollupV1: Burn already substituted"
        // );
        // require(
        //     blockHeight < burnBlockHeight,
        //     "RollupV1: block height already rolled up"
        // );
        // IERC20(usdc).transferFrom(msg.sender, address(this), amount);

        // substitutedBurns[hash] = true;

        // bool success = executeBurn(hash, amount, true);
        // require(success, "RollupV1: Burn failed");

        // // This will be returned to the msg.sender when the rollup block for it is submitted
        // substitutedBurns[hash][amount] = to;
    }

    /////////////////
    //
    // MINTS
    //
    ///////////

    // Anyone can call mint, although this is likely to be performed on behalf of the user
    // as they may not have gas to pay for the txn
    function mint(
        bytes calldata proof,
        bytes32 hash,
        bytes32 value,
        bytes32 note_kind
    ) public {
        // TODO: is this cast correct?
        if (mints[uint160(bytes20(hash))].amount != 0) {
            revert("Mint already exists");
        }

        bytes32[] memory publicInputs = new bytes32[](3);
        publicInputs[0] = value;
        publicInputs[1] = note_kind;
        publicInputs[2] = hash;

        mintVerifier.verify(proof, publicInputs);

        // TODO: is this cast correct?
        address tokenAddress = tokens[uint256(note_kind)];

        // Take the money from the external account, sender must have been previously
        // approved as per the ERC20 standard
        IERC20(tokenAddress).transferFrom(
            msg.sender,
            address(this),
            uint256(value)
        );

        // Add mint to pending mints, this still needs to be verifier with the verifyBlock,
        // but Solid validators will check that this commitment exists in the mint map before
        // accepting the mint txn into a block
        // TODO: is this cast correct?
        mints[uint160(bytes20(hash))] = Mint(
            uint256(note_kind),
            uint256(value)
        );

        emit MintAdded(hash, uint256(value), note_kind);
    }

    function mintWithAuthorization(
        bytes calldata proof,
        bytes32 hash,
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
        // TODO: is this cast correct?
        if (mints[uint160(bytes20(hash))].amount != 0) {
            revert("Mint already exists");
        }

        bytes32 structHash = keccak256(
            abi.encode(
                MINT_WITH_AUTHORIZATION_TYPE_HASH,
                hash,
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
        require(signer == from, "Invalid signer");

        // TODO: is this cast correct?
        address tokenAddress = tokens[uint256(note_kind)];

        bytes32[] memory publicInputs = new bytes32[](3);
        publicInputs[0] = value;
        publicInputs[1] = note_kind;
        publicInputs[2] = hash;
        mintVerifier.verify(proof, publicInputs);

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

        // TODO: is this cast correct?
        mints[uint160(bytes20(hash))] = Mint(
            uint256(note_kind),
            uint256(value)
        );
        emit MintAdded(hash, uint256(value), note_kind);
    }
}
