// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.20;

import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "./IUSDC.sol";

interface IAcross {
    function depositV3(
        address depositor,
        address recipient,
        address inputToken,
        address outputToken,
        uint256 inputAmount,
        uint256 outputAmount,
        uint256 destinationChainId,
        address exclusiveRelayer,
        uint32 quoteTimestamp,
        uint32 fillDeadline,
        uint32 exclusivityDeadline,
        bytes calldata message
    ) external;
}

contract AcrossWithAuthorization {
    event Deposited(address indexed depositor, bytes32 indexed nonce);

    address public immutable owner;
    address public immutable across;

    constructor(address _across, address _owner) {
        across = _across;
        owner = _owner;
    }

    function DOMAIN_SEPARATOR() public view returns (bytes32) {
        return
            keccak256(
                abi.encode(
                    keccak256(
                        "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
                    ),
                    keccak256(bytes("AcrossWithAuthorization")),
                    keccak256(bytes("1")),
                    block.chainid,
                    address(this)
                )
            );
    }

    bytes32 constant DEPOSIT_V3_WITH_AUTHORIZATION_TYPE_HASH =
        keccak256(
            "DepositV3WithAuthorization(uint256 validAfter,uint256 validBefore,bytes32 nonce,address depositor,address recipient,address inputToken,address outputToken,uint256 inputAmount,uint256 outputAmount,uint256 feeAmount,uint256 destinationChainId,address exclusiveRelayer,uint32 quoteTimestamp,uint32 fillDeadline,uint32 exclusivityDeadline,bytes message)"
        );

    function depositV3WithAuthorization(
        // signature for receiveWithAuthorization
        uint256 v,
        bytes32 r,
        bytes32 s,
        // signature for this depositV3WithAuthorization call
        uint256 v2,
        bytes32 r2,
        bytes32 s2,
        uint256 validAfter,
        uint256 validBefore,
        bytes32 nonce,
        address depositor,
        address recipient,
        address inputToken,
        address outputToken,
        uint256 inputAmount,
        uint256 outputAmount,
        uint256 feeAmount,
        uint256 destinationChainId,
        address exclusiveRelayer,
        uint32 quoteTimestamp,
        uint32 fillDeadline,
        uint32 exclusivityDeadline,
        bytes calldata message
    ) public {
        bytes32 structHash = keccak256(
            abi.encode(
                DEPOSIT_V3_WITH_AUTHORIZATION_TYPE_HASH,
                validAfter,
                validBefore,
                nonce,
                depositor,
                recipient,
                inputToken,
                outputToken,
                inputAmount,
                outputAmount,
                feeAmount,
                destinationChainId,
                exclusiveRelayer,
                quoteTimestamp,
                fillDeadline,
                exclusivityDeadline,
                keccak256(message)
            )
        );
        bytes32 computedHash = keccak256(
            abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR(), structHash)
        );
        address signer = ECDSA.recover(computedHash, uint8(v2), r2, s2);
        require(signer == depositor, "Invalid signer");

        IUSDC(inputToken).receiveWithAuthorization(
            depositor,
            address(this),
            inputAmount + feeAmount,
            validAfter,
            validBefore,
            nonce,
            uint8(v),
            r,
            s
        );

        IERC20(inputToken).approve(across, inputAmount);

        IAcross(across).depositV3(
            depositor,
            recipient,
            inputToken,
            outputToken,
            inputAmount,
            outputAmount,
            destinationChainId,
            exclusiveRelayer,
            quoteTimestamp,
            fillDeadline,
            exclusivityDeadline,
            message
        );

        emit Deposited(depositor, nonce);
    }

    function withdrawFees(address token, uint256 amount) external {
        require(msg.sender == owner, "Only owner");
        IERC20(token).transfer(owner, amount);
    }
}
