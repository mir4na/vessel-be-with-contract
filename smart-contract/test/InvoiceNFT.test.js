const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("InvoiceNFT", function () {
  let invoiceNFT;
  let owner;
  let exporter;
  let addr1;

  const sampleInvoice = {
    invoiceNumber: "INV-2024-001",
    amount: ethers.parseUnits("10000", 6), // $10,000
    advanceAmount: ethers.parseUnits("8000", 6), // $8,000 (80%)
    interestRate: 1000, // 10%
    issueDate: Math.floor(Date.now() / 1000),
    dueDate: Math.floor(Date.now() / 1000) + 60 * 24 * 60 * 60, // 60 days
    buyerCountry: "Germany",
    documentHash: "QmXxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    uri: "ipfs://QmYyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy",
  };

  beforeEach(async function () {
    [owner, exporter, addr1] = await ethers.getSigners();

    const InvoiceNFT = await ethers.getContractFactory("InvoiceNFT");
    invoiceNFT = await InvoiceNFT.deploy();
    await invoiceNFT.waitForDeployment();
  });

  describe("Deployment", function () {
    it("Should set the correct name and symbol", async function () {
      expect(await invoiceNFT.name()).to.equal("VESSEL Invoice NFT");
      expect(await invoiceNFT.symbol()).to.equal("VINV");
    });

    it("Should grant admin and minter roles to deployer", async function () {
      const DEFAULT_ADMIN_ROLE = await invoiceNFT.DEFAULT_ADMIN_ROLE();
      const MINTER_ROLE = await invoiceNFT.MINTER_ROLE();

      expect(await invoiceNFT.hasRole(DEFAULT_ADMIN_ROLE, owner.address)).to.be.true;
      expect(await invoiceNFT.hasRole(MINTER_ROLE, owner.address)).to.be.true;
    });
  });

  describe("Minting", function () {
    it("Should mint an invoice NFT", async function () {
      await invoiceNFT.mintInvoice(
        exporter.address,
        sampleInvoice.invoiceNumber,
        sampleInvoice.amount,
        sampleInvoice.advanceAmount,
        sampleInvoice.interestRate,
        sampleInvoice.issueDate,
        sampleInvoice.dueDate,
        sampleInvoice.buyerCountry,
        sampleInvoice.documentHash,
        sampleInvoice.uri
      );

      expect(await invoiceNFT.ownerOf(1)).to.equal(exporter.address);
      expect(await invoiceNFT.totalMinted()).to.equal(1);
    });

    it("Should store invoice data correctly", async function () {
      await invoiceNFT.mintInvoice(
        exporter.address,
        sampleInvoice.invoiceNumber,
        sampleInvoice.amount,
        sampleInvoice.advanceAmount,
        sampleInvoice.interestRate,
        sampleInvoice.issueDate,
        sampleInvoice.dueDate,
        sampleInvoice.buyerCountry,
        sampleInvoice.documentHash,
        sampleInvoice.uri
      );

      const invoice = await invoiceNFT.getInvoice(1);
      expect(invoice.invoiceNumber).to.equal(sampleInvoice.invoiceNumber);
      expect(invoice.amount).to.equal(sampleInvoice.amount);
      expect(invoice.advanceAmount).to.equal(sampleInvoice.advanceAmount);
      expect(invoice.buyerCountry).to.equal(sampleInvoice.buyerCountry);
    });

    it("Should not allow duplicate invoice numbers", async function () {
      await invoiceNFT.mintInvoice(
        exporter.address,
        sampleInvoice.invoiceNumber,
        sampleInvoice.amount,
        sampleInvoice.advanceAmount,
        sampleInvoice.interestRate,
        sampleInvoice.issueDate,
        sampleInvoice.dueDate,
        sampleInvoice.buyerCountry,
        sampleInvoice.documentHash,
        sampleInvoice.uri
      );

      await expect(
        invoiceNFT.mintInvoice(
          exporter.address,
          sampleInvoice.invoiceNumber,
          sampleInvoice.amount,
          sampleInvoice.advanceAmount,
          sampleInvoice.interestRate,
          sampleInvoice.issueDate,
          sampleInvoice.dueDate,
          sampleInvoice.buyerCountry,
          sampleInvoice.documentHash,
          sampleInvoice.uri
        )
      ).to.be.revertedWith("Invoice already exists");
    });

    it("Should not allow non-minters to mint", async function () {
      await expect(
        invoiceNFT.connect(addr1).mintInvoice(
          exporter.address,
          sampleInvoice.invoiceNumber,
          sampleInvoice.amount,
          sampleInvoice.advanceAmount,
          sampleInvoice.interestRate,
          sampleInvoice.issueDate,
          sampleInvoice.dueDate,
          sampleInvoice.buyerCountry,
          sampleInvoice.documentHash,
          sampleInvoice.uri
        )
      ).to.be.reverted;
    });
  });

  describe("Shipment Verification", function () {
    beforeEach(async function () {
      await invoiceNFT.mintInvoice(
        exporter.address,
        sampleInvoice.invoiceNumber,
        sampleInvoice.amount,
        sampleInvoice.advanceAmount,
        sampleInvoice.interestRate,
        sampleInvoice.issueDate,
        sampleInvoice.dueDate,
        sampleInvoice.buyerCountry,
        sampleInvoice.documentHash,
        sampleInvoice.uri
      );
    });

    it("Should verify shipment", async function () {
      await invoiceNFT.verifyShipment(1);
      const invoice = await invoiceNFT.getInvoice(1);
      expect(invoice.shipmentVerified).to.be.true;
    });

    it("Should make invoice fundable after verification", async function () {
      expect(await invoiceNFT.isFundable(1)).to.be.false;
      await invoiceNFT.verifyShipment(1);
      expect(await invoiceNFT.isFundable(1)).to.be.true;
    });

    it("Should not allow double verification", async function () {
      await invoiceNFT.verifyShipment(1);
      await expect(invoiceNFT.verifyShipment(1)).to.be.revertedWith("Already verified");
    });
  });

  describe("Status Updates", function () {
    beforeEach(async function () {
      await invoiceNFT.mintInvoice(
        exporter.address,
        sampleInvoice.invoiceNumber,
        sampleInvoice.amount,
        sampleInvoice.advanceAmount,
        sampleInvoice.interestRate,
        sampleInvoice.issueDate,
        sampleInvoice.dueDate,
        sampleInvoice.buyerCountry,
        sampleInvoice.documentHash,
        sampleInvoice.uri
      );
    });

    it("Should update status", async function () {
      await invoiceNFT.updateStatus(1, 1); // Funded
      const invoice = await invoiceNFT.getInvoice(1);
      expect(invoice.status).to.equal(1);
    });

    it("Should emit status change event", async function () {
      await expect(invoiceNFT.updateStatus(1, 1))
        .to.emit(invoiceNFT, "InvoiceStatusChanged")
        .withArgs(1, 0, 1);
    });
  });

  describe("Burning", function () {
    beforeEach(async function () {
      await invoiceNFT.mintInvoice(
        exporter.address,
        sampleInvoice.invoiceNumber,
        sampleInvoice.amount,
        sampleInvoice.advanceAmount,
        sampleInvoice.interestRate,
        sampleInvoice.issueDate,
        sampleInvoice.dueDate,
        sampleInvoice.buyerCountry,
        sampleInvoice.documentHash,
        sampleInvoice.uri
      );
    });

    it("Should burn invoice after repaid", async function () {
      await invoiceNFT.updateStatus(1, 3); // Repaid
      await invoiceNFT.burnInvoice(1, "Repaid successfully");
      await expect(invoiceNFT.ownerOf(1)).to.be.reverted;
    });

    it("Should not burn active invoice", async function () {
      await expect(invoiceNFT.burnInvoice(1, "Test")).to.be.revertedWith(
        "Invoice must be repaid or defaulted"
      );
    });
  });

  describe("Query Functions", function () {
    beforeEach(async function () {
      await invoiceNFT.mintInvoice(
        exporter.address,
        sampleInvoice.invoiceNumber,
        sampleInvoice.amount,
        sampleInvoice.advanceAmount,
        sampleInvoice.interestRate,
        sampleInvoice.issueDate,
        sampleInvoice.dueDate,
        sampleInvoice.buyerCountry,
        sampleInvoice.documentHash,
        sampleInvoice.uri
      );

      await invoiceNFT.mintInvoice(
        exporter.address,
        "INV-2024-002",
        sampleInvoice.amount,
        sampleInvoice.advanceAmount,
        sampleInvoice.interestRate,
        sampleInvoice.issueDate,
        sampleInvoice.dueDate,
        sampleInvoice.buyerCountry,
        sampleInvoice.documentHash,
        sampleInvoice.uri
      );
    });

    it("Should get exporter invoices", async function () {
      const invoices = await invoiceNFT.getExporterInvoices(exporter.address);
      expect(invoices.length).to.equal(2);
      expect(invoices[0]).to.equal(1);
      expect(invoices[1]).to.equal(2);
    });

    it("Should get token ID by invoice number", async function () {
      const tokenId = await invoiceNFT.getTokenIdByInvoiceNumber(sampleInvoice.invoiceNumber);
      expect(tokenId).to.equal(1);
    });

    it("Should return tokenURI", async function () {
      const uri = await invoiceNFT.tokenURI(1);
      expect(uri).to.equal(sampleInvoice.uri);
    });

    it("Should support ERC721 interface", async function () {
      // ERC721 interface ID
      const ERC721_INTERFACE_ID = "0x80ac58cd";
      expect(await invoiceNFT.supportsInterface(ERC721_INTERFACE_ID)).to.be.true;
    });

    it("Should return false for isFundable on non-existent token", async function () {
      expect(await invoiceNFT.isFundable(999)).to.be.false;
    });
  });

  describe("Pause/Unpause", function () {
    it("Should pause and unpause contract", async function () {
      await invoiceNFT.pause();

      await expect(
        invoiceNFT.mintInvoice(
          exporter.address,
          sampleInvoice.invoiceNumber,
          sampleInvoice.amount,
          sampleInvoice.advanceAmount,
          sampleInvoice.interestRate,
          sampleInvoice.issueDate,
          sampleInvoice.dueDate,
          sampleInvoice.buyerCountry,
          sampleInvoice.documentHash,
          sampleInvoice.uri
        )
      ).to.be.reverted;

      await invoiceNFT.unpause();

      await invoiceNFT.mintInvoice(
        exporter.address,
        sampleInvoice.invoiceNumber,
        sampleInvoice.amount,
        sampleInvoice.advanceAmount,
        sampleInvoice.interestRate,
        sampleInvoice.issueDate,
        sampleInvoice.dueDate,
        sampleInvoice.buyerCountry,
        sampleInvoice.documentHash,
        sampleInvoice.uri
      );

      expect(await invoiceNFT.totalMinted()).to.equal(1);
    });

    it("Should not allow non-admin to pause", async function () {
      await expect(invoiceNFT.connect(addr1).pause()).to.be.reverted;
    });

    it("Should not allow non-admin to unpause", async function () {
      await invoiceNFT.pause();
      await expect(invoiceNFT.connect(addr1).unpause()).to.be.reverted;
    });
  });

  describe("Input Validation", function () {
    it("Should reject empty invoice number", async function () {
      await expect(
        invoiceNFT.mintInvoice(
          exporter.address,
          "", // empty
          sampleInvoice.amount,
          sampleInvoice.advanceAmount,
          sampleInvoice.interestRate,
          sampleInvoice.issueDate,
          sampleInvoice.dueDate,
          sampleInvoice.buyerCountry,
          sampleInvoice.documentHash,
          sampleInvoice.uri
        )
      ).to.be.revertedWith("Invoice number required");
    });

    it("Should reject zero amount", async function () {
      await expect(
        invoiceNFT.mintInvoice(
          exporter.address,
          sampleInvoice.invoiceNumber,
          0, // zero amount
          sampleInvoice.advanceAmount,
          sampleInvoice.interestRate,
          sampleInvoice.issueDate,
          sampleInvoice.dueDate,
          sampleInvoice.buyerCountry,
          sampleInvoice.documentHash,
          sampleInvoice.uri
        )
      ).to.be.revertedWith("Amount must be positive");
    });

    it("Should reject advance amount exceeding total amount", async function () {
      await expect(
        invoiceNFT.mintInvoice(
          exporter.address,
          sampleInvoice.invoiceNumber,
          sampleInvoice.amount,
          sampleInvoice.amount + BigInt(1000), // advance > amount
          sampleInvoice.interestRate,
          sampleInvoice.issueDate,
          sampleInvoice.dueDate,
          sampleInvoice.buyerCountry,
          sampleInvoice.documentHash,
          sampleInvoice.uri
        )
      ).to.be.revertedWith("Advance cannot exceed amount");
    });

    it("Should reject due date before issue date", async function () {
      await expect(
        invoiceNFT.mintInvoice(
          exporter.address,
          sampleInvoice.invoiceNumber,
          sampleInvoice.amount,
          sampleInvoice.advanceAmount,
          sampleInvoice.interestRate,
          sampleInvoice.dueDate, // swapped
          sampleInvoice.issueDate, // swapped
          sampleInvoice.buyerCountry,
          sampleInvoice.documentHash,
          sampleInvoice.uri
        )
      ).to.be.revertedWith("Due date must be after issue date");
    });
  });

  describe("Burning with Defaulted Status", function () {
    beforeEach(async function () {
      await invoiceNFT.mintInvoice(
        exporter.address,
        sampleInvoice.invoiceNumber,
        sampleInvoice.amount,
        sampleInvoice.advanceAmount,
        sampleInvoice.interestRate,
        sampleInvoice.issueDate,
        sampleInvoice.dueDate,
        sampleInvoice.buyerCountry,
        sampleInvoice.documentHash,
        sampleInvoice.uri
      );
    });

    it("Should burn invoice after defaulted", async function () {
      await invoiceNFT.updateStatus(1, 4); // Defaulted
      await invoiceNFT.burnInvoice(1, "Defaulted - written off");
      await expect(invoiceNFT.ownerOf(1)).to.be.reverted;
    });
  });

  describe("Oracle Role", function () {
    it("Should not allow non-oracle to verify shipment", async function () {
      await invoiceNFT.mintInvoice(
        exporter.address,
        sampleInvoice.invoiceNumber,
        sampleInvoice.amount,
        sampleInvoice.advanceAmount,
        sampleInvoice.interestRate,
        sampleInvoice.issueDate,
        sampleInvoice.dueDate,
        sampleInvoice.buyerCountry,
        sampleInvoice.documentHash,
        sampleInvoice.uri
      );

      await expect(invoiceNFT.connect(addr1).verifyShipment(1)).to.be.reverted;
    });
  });
});
