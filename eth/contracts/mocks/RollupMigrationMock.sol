pragma solidity ^0.8.20;

contract RollupMigrationMock {
    struct Mint {
        bytes32 note_kind;
        uint256 amount;
        bool spent;
    }

    mapping(bytes32 => Mint) public mints;

    event MintAdded(bytes32 indexed mint_hash, uint256 value, bytes32 note_kind);
    event Minted(bytes32 indexed hash, bytes32 value, bytes32 note_kind);

    function getMint(bytes32 hash) external view returns (Mint memory) {
        return mints[hash];
    }

    function emitMintAdded(bytes32 mint_hash, uint256 value, bytes32 note_kind) external {
        mints[mint_hash] = Mint({
            note_kind: note_kind,
            amount: value,
            spent: false
        });
        emit MintAdded(mint_hash, value, note_kind);
    }

    function emitMinted(bytes32 hash, bytes32 value, bytes32 note_kind) external {
        mints[hash].spent = true;
        emit Minted(hash, value, note_kind);
    }

    function mint(bytes32 mint_hash, bytes32 value, bytes32 note_kind) external {
        mints[mint_hash] = Mint({
            note_kind: note_kind,
            amount: uint256(value),
            spent: false
        });
        emit MintAdded(mint_hash, uint256(value), note_kind);
    }
}