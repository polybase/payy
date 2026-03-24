// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";

contract LitActionRegistry is Ownable {
    string public childLitActionCID;

    event ChildLitActionCIDUpdated(string oldCID, string newCID);

    constructor(address initialOwner) Ownable(initialOwner) {}

    function setChildLitActionCID(string memory newCID) external onlyOwner {
        require(bytes(newCID).length > 0, "Child Lit Action CID cannot be empty");
        string memory oldCID = childLitActionCID;
        childLitActionCID = newCID;
        emit ChildLitActionCIDUpdated(oldCID, newCID);
    }

    function getChildLitActionCID() external view returns (string memory) {
        return childLitActionCID;
    }

    // Alias for compatibility with parent lit action
    function getChildIPFSCID() external view returns (string memory) {
        return childLitActionCID;
    }
}
