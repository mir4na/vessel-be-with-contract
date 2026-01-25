// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

import "./InvoiceNFT.sol";

/**
 * @title InvoicePool
 * @dev Manages funding pools for invoice NFTs on VESSEL platform
 * NOTE: This contract uses ABSTRACTED payments - amounts are recorded on-chain
 * but actual payments are handled off-chain. No ERC20 token transfers.
 * This provides blockchain transparency without requiring token integration.
 */
contract InvoicePool is AccessControl, ReentrancyGuard, Pausable {
    using SafeERC20 for IERC20;
    bytes32 public constant OPERATOR_ROLE = keccak256("OPERATOR_ROLE");

    // Contracts
    InvoiceNFT public invoiceNFT;
    IERC20 public idrxToken;

    // Platform fee in basis points (e.g., 200 = 2%)
    uint256 public platformFeeBps = 200;
    address public platformWallet;

    // Pool status enum
    enum PoolStatus {
        Open,
        Filled,
        Disbursed,
        Closed,
        Defaulted
    }

    // Pool structure
    struct Pool {
        uint256 tokenId;
        uint256 targetAmount;
        uint256 fundedAmount;
        uint256 investorCount;
        uint256 interestRate;
        uint256 dueDate;
        address exporter;
        PoolStatus status;
        uint256 openedAt;
        uint256 filledAt;
        uint256 disbursedAt;
        uint256 closedAt;
    }

    // Investment structure - records investment details on-chain for transparency
    struct Investment {
        address investor;
        uint256 amount;
        uint256 expectedReturn;
        uint256 actualReturn;
        bool claimed;
        uint256 investedAt;
    }

    // Mappings
    mapping(uint256 => Pool) public pools; // tokenId => Pool
    mapping(uint256 => Investment[]) public poolInvestments; // tokenId => investments
    mapping(address => uint256[]) public investorPools; // investor => tokenIds they invested in

    // Events - All transactions are recorded on-chain via events
    event PoolCreated(
        uint256 indexed tokenId,
        uint256 targetAmount,
        uint256 interestRate
    );
    event InvestmentRecorded(
        uint256 indexed tokenId,
        address indexed investor,
        uint256 amount,
        uint256 expectedReturn
    );
    event PoolFilled(
        uint256 indexed tokenId,
        uint256 totalAmount,
        uint256 investorCount
    );
    event DisbursementRecorded(
        uint256 indexed tokenId,
        address indexed exporter,
        uint256 amount
    );
    event RepaymentRecorded(uint256 indexed tokenId, uint256 amount);
    event ExcessRepaymentRecorded(
        uint256 indexed tokenId,
        address indexed recipient,
        uint256 amount
    );
    event InvestorReturnRecorded(
        uint256 indexed tokenId,
        address indexed investor,
        uint256 amount
    );
    event PoolClosed(uint256 indexed tokenId);
    event PoolDefaulted(uint256 indexed tokenId);

    // ... (existing code) ...

    /**
     * @dev Record excess repayment (e.g. to mitra)
     */
    function recordExcessRepayment(
        uint256 tokenId,
        address recipient,
        uint256 amount
    ) external onlyRole(OPERATOR_ROLE) {
        if (amount > 0) {
            idrxToken.safeTransfer(recipient, amount);
        }
        emit ExcessRepaymentRecorded(tokenId, recipient, amount);
    }

    constructor(address _invoiceNFT, address _platformWallet, address _idrxToken) {
        invoiceNFT = InvoiceNFT(_invoiceNFT);
        platformWallet = _platformWallet;
        idrxToken = IERC20(_idrxToken);

        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(OPERATOR_ROLE, msg.sender);
    }

    /**
     * @dev Create a funding pool for an invoice
     */
    function createPool(
        uint256 tokenId
    ) external onlyRole(OPERATOR_ROLE) whenNotPaused {
        require(invoiceNFT.isFundable(tokenId), "Invoice not fundable");
        require(pools[tokenId].targetAmount == 0, "Pool already exists");

        InvoiceNFT.Invoice memory invoice = invoiceNFT.getInvoice(tokenId);

        pools[tokenId] = Pool({
            tokenId: tokenId,
            targetAmount: invoice.advanceAmount,
            fundedAmount: 0,
            investorCount: 0,
            interestRate: invoice.interestRate,
            dueDate: invoice.dueDate,
            exporter: invoice.exporter,
            status: PoolStatus.Open,
            openedAt: block.timestamp,
            filledAt: 0,
            disbursedAt: 0,
            closedAt: 0
        });

        // Update NFT status
        invoiceNFT.updateStatus(tokenId, InvoiceNFT.InvoiceStatus.Funded);

        emit PoolCreated(tokenId, invoice.advanceAmount, invoice.interestRate);
    }

    /**
     * @dev Record an investment (called by operator after off-chain payment is verified)
     * NOTE: No actual token transfer - this only records the investment on-chain
     * @param tokenId The pool token ID
     * @param investor The investor's wallet address
     * @param amount The investment amount (in smallest unit)
     */
    function recordInvestment(
        uint256 tokenId,
        address investor,
        uint256 amount
    ) external onlyRole(OPERATOR_ROLE) nonReentrant whenNotPaused {
        Pool storage pool = pools[tokenId];
        require(pool.targetAmount > 0, "Pool does not exist");
        require(pool.status == PoolStatus.Open, "Pool not open");
        require(amount > 0, "Amount must be positive");
        require(
            idrxToken.balanceOf(address(this)) >= pool.fundedAmount + amount,
            "Insufficient pool balance"
        );

        uint256 remaining = pool.targetAmount - pool.fundedAmount;
        require(amount <= remaining, "Amount exceeds remaining capacity");

        // Calculate expected return based on interest rate and time
        uint256 daysToMaturity = (pool.dueDate - block.timestamp) / 1 days;
        if (daysToMaturity == 0) daysToMaturity = 1;
        uint256 expectedReturn = amount +
            (amount * pool.interestRate * daysToMaturity) /
            (365 * 10000);

        // Record investment on-chain (no token transfer)
        poolInvestments[tokenId].push(
            Investment({
                investor: investor,
                amount: amount,
                expectedReturn: expectedReturn,
                actualReturn: 0,
                claimed: false,
                investedAt: block.timestamp
            })
        );

        pool.fundedAmount += amount;
        pool.investorCount++;
        investorPools[investor].push(tokenId);

        emit InvestmentRecorded(tokenId, investor, amount, expectedReturn);

        // Check if pool is filled
        if (pool.fundedAmount >= pool.targetAmount) {
            pool.status = PoolStatus.Filled;
            pool.filledAt = block.timestamp;
            emit PoolFilled(tokenId, pool.fundedAmount, pool.investorCount);
        }
    }

    /**
     * @dev Record disbursement to exporter (called after off-chain payment is made)
     * NOTE: No actual token transfer - this only records the disbursement on-chain
     */
    function recordDisbursement(
        uint256 tokenId
    ) external onlyRole(OPERATOR_ROLE) nonReentrant {
        Pool storage pool = pools[tokenId];
        require(pool.status == PoolStatus.Filled, "Pool not filled");

        pool.status = PoolStatus.Disbursed;
        pool.disbursedAt = block.timestamp;

        uint256 feeAmount = (pool.fundedAmount * platformFeeBps) / 10000;
        uint256 disbursementAmount = pool.fundedAmount - feeAmount;

        if (feeAmount > 0) {
            idrxToken.safeTransfer(platformWallet, feeAmount);
        }
        if (disbursementAmount > 0) {
            idrxToken.safeTransfer(pool.exporter, disbursementAmount);
        }

        emit DisbursementRecorded(tokenId, pool.exporter, pool.fundedAmount);
    }

    /**
     * @dev Record repayment and investor returns (called after importer pays off-chain)
     * NOTE: No actual token transfer - this only records the repayment on-chain
     * @param tokenId The pool token ID
     * @param totalAmount Total amount repaid by importer
     * @param investorReturns Array of actual returns for each investor (in order of investments)
     */
    function recordRepayment(
        uint256 tokenId,
        uint256 totalAmount,
        uint256[] calldata investorReturns
    ) external onlyRole(OPERATOR_ROLE) nonReentrant {
        Pool storage pool = pools[tokenId];
        require(pool.status == PoolStatus.Disbursed, "Pool not disbursed");

        Investment[] storage investments = poolInvestments[tokenId];
        require(
            investorReturns.length == investments.length,
            "Invalid returns array length"
        );

        uint256 feeAmount = (totalAmount * platformFeeBps) / 10000;
        uint256 remainingAmount = totalAmount - feeAmount;
        uint256 totalPaid;

        // Record each investor's return
        for (uint256 i = 0; i < investments.length; i++) {
            Investment storage inv = investments[i];
            inv.actualReturn = investorReturns[i];
            inv.claimed = true;
            totalPaid += investorReturns[i];
            if (investorReturns[i] > 0) {
                idrxToken.safeTransfer(inv.investor, investorReturns[i]);
            }

            emit InvestorReturnRecorded(
                tokenId,
                inv.investor,
                investorReturns[i]
            );
        }

        if (feeAmount > 0) {
            idrxToken.safeTransfer(platformWallet, feeAmount);
        }
        if (remainingAmount > totalPaid) {
            idrxToken.safeTransfer(pool.exporter, remainingAmount - totalPaid);
        }

        pool.status = PoolStatus.Closed;
        pool.closedAt = block.timestamp;

        // Update NFT status
        invoiceNFT.updateStatus(tokenId, InvoiceNFT.InvoiceStatus.Repaid);

        emit RepaymentRecorded(tokenId, totalAmount);
        emit PoolClosed(tokenId);
    }

    /**
     * @dev Admin closes a pool early (stops accepting investments, refunds are handled off-chain)
     */
    function closePoolEarly(uint256 tokenId) external onlyRole(OPERATOR_ROLE) {
        Pool storage pool = pools[tokenId];
        require(pool.targetAmount > 0, "Pool does not exist");
        require(pool.status == PoolStatus.Open || pool.status == PoolStatus.Filled, "Pool not active");

        pool.status = PoolStatus.Closed;
        pool.closedAt = block.timestamp;

        emit PoolClosed(tokenId);
    }

    /**
     * @dev Mark pool as defaulted
     */
    function markDefaulted(uint256 tokenId) external onlyRole(OPERATOR_ROLE) {
        Pool storage pool = pools[tokenId];
        require(pool.status == PoolStatus.Disbursed, "Pool not disbursed");
        require(
            block.timestamp > pool.dueDate + 30 days,
            "Grace period not passed"
        );

        pool.status = PoolStatus.Defaulted;

        // Update NFT status
        invoiceNFT.updateStatus(tokenId, InvoiceNFT.InvoiceStatus.Defaulted);

        emit PoolDefaulted(tokenId);
    }

    /**
     * @dev Get pool details
     */
    function getPool(uint256 tokenId) external view returns (Pool memory) {
        return pools[tokenId];
    }

    /**
     * @dev Get investments for a pool
     */
    function getPoolInvestments(
        uint256 tokenId
    ) external view returns (Investment[] memory) {
        return poolInvestments[tokenId];
    }

    /**
     * @dev Get pools an investor has invested in
     */
    function getInvestorPools(
        address investor
    ) external view returns (uint256[] memory) {
        return investorPools[investor];
    }

    /**
     * @dev Get remaining capacity in pool
     */
    function getRemainingCapacity(
        uint256 tokenId
    ) external view returns (uint256) {
        Pool memory pool = pools[tokenId];
        if (pool.status != PoolStatus.Open) return 0;
        return pool.targetAmount - pool.fundedAmount;
    }

    /**
     * @dev Update platform fee
     */
    function setPlatformFee(
        uint256 newFeeBps
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(newFeeBps <= 1000, "Fee too high"); // Max 10%
        platformFeeBps = newFeeBps;
    }

    /**
     * @dev Update platform wallet
     */
    function setPlatformWallet(
        address newWallet
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(newWallet != address(0), "Invalid address");
        platformWallet = newWallet;
    }

    /**
     * @dev Pause/Unpause
     */
    function pause() external onlyRole(DEFAULT_ADMIN_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(DEFAULT_ADMIN_ROLE) {
        _unpause();
    }
}
