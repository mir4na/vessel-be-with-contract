// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title MockIDRX
 * @dev Mock IDRX token for testing on Base Sepolia
 * In production, use the real IDRX token contract
 */
contract MockIDRX is ERC20, Ownable {
    uint8 private _decimals = 2; // IDRX uses 2 decimals like IDR

    constructor() ERC20("Mock IDRX Token", "IDRX") Ownable(msg.sender) {
        // Mint initial supply to deployer (1 billion IDRX)
        _mint(msg.sender, 1_000_000_000 * 10**_decimals);
    }

    function decimals() public view override returns (uint8) {
        return _decimals;
    }

    /**
     * @dev Mint tokens (for testing only)
     */
    function mint(address to, uint256 amount) external onlyOwner {
        _mint(to, amount);
    }

    /**
     * @dev Faucet function - anyone can get 10,000 IDRX for testing
     */
    function faucet() external {
        _mint(msg.sender, 10_000 * 10**_decimals);
    }
}
