// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";

contract SocialRecovery is Ownable {

    struct GuardianConfig {
        uint256 threshold;
        bool enabled;
        bytes32[] guardianCIDs;
        mapping(bytes32 => bytes) guardianEntries;
    }


    mapping(address => GuardianConfig) public guardianConfigs;

    event GuardianCIDAdded(address indexed user, string guardianCID);

    constructor(address initialOwner) Ownable(initialOwner) {}

    function addGuardianCID(address user, string memory guardianCID, string memory guardianValue) external onlyOwner {
        GuardianConfig storage guardianConfig = guardianConfigs[user];

        if (guardianConfig.guardianCIDs.length == 0) {
            guardianConfig.enabled = true;
        }

        require(guardianConfig.enabled, "Guardian recovery disabled");
        require(bytes(guardianCID).length > 0, "Guardian CID cannot be empty");
        require(bytes(guardianValue).length > 0, "Guardian value cannot be empty");

        bytes32 cidHash = keccak256(bytes(guardianCID));

        bool exists = guardianConfig.guardianEntries[cidHash].length > 0;

        require(!exists, "Guardian Already exists");

        guardianConfig.guardianEntries[cidHash] = bytes(guardianValue);
        guardianConfig.guardianCIDs.push(cidHash);


        if (guardianConfig.threshold == 0) {
            guardianConfig.threshold = 1;
        }

        emit GuardianCIDAdded(user, guardianCID);
    }

    function updateThreshold(address user, uint256 newThreshold) external onlyOwner {
        GuardianConfig storage config = guardianConfigs[user];
        require(config.guardianCIDs.length > 0, "No guardians configured");
        require(newThreshold > 0 && newThreshold <= config.guardianCIDs.length, "Invalid threshold");
        config.threshold = newThreshold;
    }


    function getGuardianConfig(address user) external view returns (
        uint256 threshold,
        bool enabled,
        uint256 guardianCount,
        bytes32[] memory guardianCIDs
    ) {
        GuardianConfig storage config = guardianConfigs[user];
        return (
            config.threshold,
            config.enabled,
            config.guardianCIDs.length,
            config.guardianCIDs  
        );
    }

    /// @notice Get the auth_value for a specific guardian CID hash
    function getGuardianEntry(address user, bytes32 cidHash) external view returns (bytes memory) {
        return guardianConfigs[user].guardianEntries[cidHash];
    }

}
