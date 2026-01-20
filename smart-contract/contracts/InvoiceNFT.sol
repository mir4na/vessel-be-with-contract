// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721URIStorage.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Burnable.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";

/**
 * @title InvoiceNFT
 * @dev NFT representing tokenized invoices for the VESSEL platform
 * Each NFT represents a real-world invoice that can be used as collateral for funding
 */
contract InvoiceNFT is
    ERC721,
    ERC721URIStorage,
    ERC721Burnable,
    AccessControl,
    Pausable
{
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 public constant ORACLE_ROLE = keccak256("ORACLE_ROLE");

    uint256 private _tokenIdCounter;

    // Invoice status enum
    enum InvoiceStatus {
        Active,
        Funded,
        Matured,
        Repaid,
        Defaulted
    }

    // Invoice data structure
    struct Invoice {
        string invoiceNumber;
        uint256 amount;
        uint256 advanceAmount;
        uint256 interestRate; // Basis points (e.g., 1000 = 10%)
        uint256 issueDate;
        uint256 dueDate;
        address exporter;
        string buyerCountry;
        string documentHash;
        InvoiceStatus status;
        bool shipmentVerified;
    }

    // Mappings
    mapping(uint256 => Invoice) public invoices;
    mapping(string => uint256) public invoiceNumberToTokenId;
    mapping(address => uint256[]) public exporterInvoices;

    // Events
    event InvoiceMinted(
        uint256 indexed tokenId,
        address indexed exporter,
        string invoiceNumber,
        uint256 amount,
        uint256 dueDate
    );
    event InvoiceStatusChanged(
        uint256 indexed tokenId,
        InvoiceStatus oldStatus,
        InvoiceStatus newStatus
    );
    event ShipmentVerified(uint256 indexed tokenId, address verifier);
    event InvoiceBurned(uint256 indexed tokenId, string reason);

    constructor() ERC721("VESSEL Invoice NFT", "VINV") {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(MINTER_ROLE, msg.sender);
        _grantRole(ORACLE_ROLE, msg.sender);
    }

    /**
     * @dev Mint a new invoice NFT
     * @param to Address of the exporter receiving the NFT
     * @param invoiceNumber Unique invoice identifier
     * @param amount Total invoice amount in smallest unit (e.g., cents for USD)
     * @param advanceAmount Amount to be advanced to exporter
     * @param interestRate Interest rate in basis points
     * @param issueDate Invoice issue date (Unix timestamp)
     * @param dueDate Invoice due date (Unix timestamp)
     * @param buyerCountry Country of the buyer
     * @param documentHash IPFS hash of invoice documents
     * @param uri Metadata URI for the NFT
     */
    function mintInvoice(
        address to,
        string memory invoiceNumber,
        uint256 amount,
        uint256 advanceAmount,
        uint256 interestRate,
        uint256 issueDate,
        uint256 dueDate,
        string memory buyerCountry,
        string memory documentHash,
        string memory uri
    ) external onlyRole(MINTER_ROLE) whenNotPaused returns (uint256) {
        require(bytes(invoiceNumber).length > 0, "Invoice number required");
        require(amount > 0, "Amount must be positive");
        require(advanceAmount <= amount, "Advance cannot exceed amount");
        require(dueDate > issueDate, "Due date must be after issue date");
        require(
            invoiceNumberToTokenId[invoiceNumber] == 0,
            "Invoice already exists"
        );

        _tokenIdCounter++;
        uint256 tokenId = _tokenIdCounter;

        _safeMint(to, tokenId);
        _setTokenURI(tokenId, uri);

        invoices[tokenId] = Invoice({
            invoiceNumber: invoiceNumber,
            amount: amount,
            advanceAmount: advanceAmount,
            interestRate: interestRate,
            issueDate: issueDate,
            dueDate: dueDate,
            exporter: to,
            buyerCountry: buyerCountry,
            documentHash: documentHash,
            status: InvoiceStatus.Active,
            shipmentVerified: false
        });

        invoiceNumberToTokenId[invoiceNumber] = tokenId;
        exporterInvoices[to].push(tokenId);

        emit InvoiceMinted(tokenId, to, invoiceNumber, amount, dueDate);

        return tokenId;
    }

    /**
     * @dev Verify shipment for an invoice (called by oracle)
     */
    function verifyShipment(
        uint256 tokenId
    ) external onlyRole(ORACLE_ROLE) whenNotPaused {
        require(_exists(tokenId), "Token does not exist");
        require(!invoices[tokenId].shipmentVerified, "Already verified");

        invoices[tokenId].shipmentVerified = true;

        emit ShipmentVerified(tokenId, msg.sender);
    }

    /**
     * @dev Update invoice status
     */
    function updateStatus(
        uint256 tokenId,
        InvoiceStatus newStatus
    ) external onlyRole(MINTER_ROLE) whenNotPaused {
        require(_exists(tokenId), "Token does not exist");

        InvoiceStatus oldStatus = invoices[tokenId].status;
        require(oldStatus != newStatus, "Status unchanged");

        invoices[tokenId].status = newStatus;

        emit InvoiceStatusChanged(tokenId, oldStatus, newStatus);
    }

    /**
     * @dev Burn invoice NFT (after repayment or default resolution)
     */
    function burnInvoice(
        uint256 tokenId,
        string memory reason
    ) external onlyRole(MINTER_ROLE) {
        require(_exists(tokenId), "Token does not exist");

        Invoice storage invoice = invoices[tokenId];
        require(
            invoice.status == InvoiceStatus.Repaid ||
                invoice.status == InvoiceStatus.Defaulted,
            "Invoice must be repaid or defaulted"
        );

        _burn(tokenId);

        emit InvoiceBurned(tokenId, reason);
    }

    /**
     * @dev Get invoice details
     */
    function getInvoice(
        uint256 tokenId
    ) external view returns (Invoice memory) {
        require(_exists(tokenId), "Token does not exist");
        return invoices[tokenId];
    }

    /**
     * @dev Get all invoices for an exporter
     */
    function getExporterInvoices(
        address exporter
    ) external view returns (uint256[] memory) {
        return exporterInvoices[exporter];
    }

    /**
     * @dev Check if invoice is fundable
     */
    function isFundable(uint256 tokenId) external view returns (bool) {
        if (!_exists(tokenId)) return false;
        Invoice memory invoice = invoices[tokenId];
        return
            invoice.status == InvoiceStatus.Active && invoice.shipmentVerified;
    }

    /**
     * @dev Get token ID by invoice number
     */
    function getTokenIdByInvoiceNumber(
        string memory invoiceNumber
    ) external view returns (uint256) {
        return invoiceNumberToTokenId[invoiceNumber];
    }

    /**
     * @dev Get total minted count
     */
    function totalMinted() external view returns (uint256) {
        return _tokenIdCounter;
    }

    // Required overrides for OZ v5
    function tokenURI(
        uint256 tokenId
    ) public view override(ERC721, ERC721URIStorage) returns (string memory) {
        return super.tokenURI(tokenId);
    }

    function supportsInterface(
        bytes4 interfaceId
    )
        public
        view
        override(ERC721, ERC721URIStorage, AccessControl)
        returns (bool)
    {
        return super.supportsInterface(interfaceId);
    }

    function _exists(uint256 tokenId) internal view returns (bool) {
        return _ownerOf(tokenId) != address(0);
    }

    function pause() external onlyRole(DEFAULT_ADMIN_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(DEFAULT_ADMIN_ROLE) {
        _unpause();
    }
}
