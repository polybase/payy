// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

// https://polygonscan.com/address/0x235ae97b28466db30469b89a9fe4cff0659f82cb#code
interface IUSDC is IERC20 {
    function DOMAIN_SEPARATOR() external view returns (bytes32);

    function initialize(
        string memory tokenName,
        string memory tokenSymbol,
        string memory tokenCurrency,
        uint8 tokenDecimals,
        address newMasterMinter,
        address newPauser,
        address newBlacklister,
        address newOwner
    ) external;

    function initializeV2(string calldata newName) external;

    function initializeV2_1(address lostAndFound) external;

    function configureMinter(
        address minter,
        uint256 minterAllowedAmount
    ) external returns (bool);

    function mint(address _to, uint256 _amount) external returns (bool);

    function receiveWithAuthorization(
        address from,
        address to,
        uint256 value,
        uint256 validAfter,
        uint256 validBefore,
        bytes32 nonce,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external;

    function transferWithAuthorization(
        address from,
        address to,
        uint256 value,
        uint256 validAfter,
        uint256 validBefore,
        bytes32 nonce,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external;

    function isBlacklisted(address _account) external view returns (bool);
}
