// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "../IUSDC.sol";

contract MockUSDC is IUSDC {
    mapping(address => uint256) private _balances;
    mapping(address => mapping(address => uint256)) private _allowances;
    mapping(address => bool) private _blacklisted;
    
    uint256 private _totalSupply;
    string private _name = "Mock USDC";
    string private _symbol = "MUSDC";

    constructor() {
        // Give some initial balance to deployer for testing
        _totalSupply = 1_000_000 * 10**6; // 1M USDC with 6 decimals
        _balances[msg.sender] = _totalSupply;
    }

    // ERC20 functions
    function totalSupply() external view returns (uint256) {
        return _totalSupply;
    }

    function balanceOf(address account) external view returns (uint256) {
        return _balances[account];
    }

    function transfer(address to, uint256 amount) external returns (bool) {
        require(_balances[msg.sender] >= amount, "Insufficient balance");
        _balances[msg.sender] -= amount;
        _balances[to] += amount;
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        require(_allowances[from][msg.sender] >= amount, "Insufficient allowance");
        require(_balances[from] >= amount, "Insufficient balance");
        _allowances[from][msg.sender] -= amount;
        _balances[from] -= amount;
        _balances[to] += amount;
        return true;
    }

    function approve(address spender, uint256 amount) external returns (bool) {
        _allowances[msg.sender][spender] = amount;
        return true;
    }

    function allowance(address owner, address spender) external view returns (uint256) {
        return _allowances[owner][spender];
    }

    // USDC-specific functions
    function DOMAIN_SEPARATOR() external pure returns (bytes32) {
        return keccak256("MOCK_USDC_DOMAIN");
    }

    function initialize(
        string memory,
        string memory,
        string memory,
        uint8,
        address,
        address,
        address,
        address
    ) external {
        // Mock implementation - do nothing
    }

    function initializeV2(string calldata) external {
        // Mock implementation - do nothing
    }

    function initializeV2_1(address) external {
        // Mock implementation - do nothing
    }

    function configureMinter(address, uint256) external returns (bool) {
        return true;
    }

    function mint(address _to, uint256 _amount) external returns (bool) {
        _balances[_to] += _amount;
        _totalSupply += _amount;
        return true;
    }

    function receiveWithAuthorization(
        address from,
        address to,
        uint256 value,
        uint256,
        uint256,
        bytes32,
        uint8,
        bytes32,
        bytes32
    ) external override {
        require(_balances[from] >= value, "Insufficient balance");
        _balances[from] -= value;
        _balances[to] += value;
    }

    function transferWithAuthorization(
        address from,
        address to,
        uint256 value,
        uint256,
        uint256,
        bytes32,
        uint8,
        bytes32,
        bytes32
    ) external {
        require(_balances[from] >= value, "Insufficient balance");
        _balances[from] -= value;
        _balances[to] += value;
    }

    function isBlacklisted(address _account) external view returns (bool) {
        return _blacklisted[_account];
    }
}