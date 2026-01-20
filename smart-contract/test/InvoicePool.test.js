const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("InvoicePool", function () {
  let invoiceNFT;
  let invoicePool;
  let owner;
  let exporter;
  let investor1;
  let investor2;

  // Using simple amounts (no decimals needed for abstracted payments)
  const sampleInvoice = {
    invoiceNumber: "INV-2024-001",
    amount: ethers.parseEther("10000"),
    advanceAmount: ethers.parseEther("8000"),
    interestRate: 1000, // 10%
    issueDate: Math.floor(Date.now() / 1000),
    dueDate: Math.floor(Date.now() / 1000) + 60 * 24 * 60 * 60,
    buyerCountry: "Germany",
    documentHash: "QmXxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    uri: "ipfs://QmYyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy",
  };

  beforeEach(async function () {
    [owner, exporter, investor1, investor2] = await ethers.getSigners();

    // Deploy InvoiceNFT
    const InvoiceNFT = await ethers.getContractFactory("InvoiceNFT");
    invoiceNFT = await InvoiceNFT.deploy();
    await invoiceNFT.waitForDeployment();

    // Deploy InvoicePool (no stablecoin needed - abstracted payments)
    const InvoicePool = await ethers.getContractFactory("InvoicePool");
    invoicePool = await InvoicePool.deploy(
      await invoiceNFT.getAddress(),
      owner.address // platform wallet
    );
    await invoicePool.waitForDeployment();

    // Grant roles
    const MINTER_ROLE = await invoiceNFT.MINTER_ROLE();
    await invoiceNFT.grantRole(MINTER_ROLE, await invoicePool.getAddress());
  });

  async function mintAndVerifyInvoice() {
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
    await invoiceNFT.verifyShipment(1);
    return 1;
  }

  describe("Pool Creation", function () {
    it("Should create a funding pool", async function () {
      const tokenId = await mintAndVerifyInvoice();
      await invoicePool.createPool(tokenId);

      const pool = await invoicePool.getPool(tokenId);
      expect(pool.targetAmount).to.equal(sampleInvoice.advanceAmount);
      expect(pool.status).to.equal(0); // Open
    });

    it("Should not create pool for unverified invoice", async function () {
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

      await expect(invoicePool.createPool(1)).to.be.revertedWith("Invoice not fundable");
    });

    it("Should not allow non-operator to create pool", async function () {
      await mintAndVerifyInvoice();
      await expect(invoicePool.connect(investor1).createPool(1)).to.be.reverted;
    });
  });

  describe("Investment Recording", function () {
    beforeEach(async function () {
      await mintAndVerifyInvoice();
      await invoicePool.createPool(1);
    });

    it("Should record investment", async function () {
      const investAmount = ethers.parseEther("5000");
      await invoicePool.recordInvestment(1, investor1.address, investAmount);

      const pool = await invoicePool.getPool(1);
      expect(pool.fundedAmount).to.equal(investAmount);
      expect(pool.investorCount).to.equal(1);
    });

    it("Should track investments correctly", async function () {
      const investAmount = ethers.parseEther("5000");
      await invoicePool.recordInvestment(1, investor1.address, investAmount);

      const investments = await invoicePool.getPoolInvestments(1);
      expect(investments.length).to.equal(1);
      expect(investments[0].investor).to.equal(investor1.address);
      expect(investments[0].amount).to.equal(investAmount);
    });

    it("Should fill pool when target reached", async function () {
      await invoicePool.recordInvestment(1, investor1.address, ethers.parseEther("5000"));
      await invoicePool.recordInvestment(1, investor2.address, ethers.parseEther("3000"));

      const pool = await invoicePool.getPool(1);
      expect(pool.status).to.equal(1); // Filled
    });

    it("Should not allow over-investment", async function () {
      await expect(
        invoicePool.recordInvestment(1, investor1.address, ethers.parseEther("10000"))
      ).to.be.revertedWith("Amount exceeds remaining capacity");
    });

    it("Should not allow zero investment", async function () {
      await expect(
        invoicePool.recordInvestment(1, investor1.address, 0)
      ).to.be.revertedWith("Amount must be positive");
    });

    it("Should not allow investment in non-existent pool", async function () {
      await expect(
        invoicePool.recordInvestment(999, investor1.address, ethers.parseEther("1000"))
      ).to.be.revertedWith("Pool does not exist");
    });
  });

  describe("Disbursement Recording", function () {
    beforeEach(async function () {
      await mintAndVerifyInvoice();
      await invoicePool.createPool(1);
      await invoicePool.recordInvestment(1, investor1.address, ethers.parseEther("8000"));
    });

    it("Should record disbursement to exporter", async function () {
      await expect(invoicePool.recordDisbursement(1))
        .to.emit(invoicePool, "DisbursementRecorded")
        .withArgs(1, exporter.address, ethers.parseEther("8000"));
    });

    it("Should update pool status after disbursement", async function () {
      await invoicePool.recordDisbursement(1);
      const pool = await invoicePool.getPool(1);
      expect(pool.status).to.equal(2); // Disbursed
    });

    it("Should not disburse unfilled pool", async function () {
      // Create a new invoice with unfilled pool
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
      await invoiceNFT.verifyShipment(2);
      await invoicePool.createPool(2);
      await invoicePool.recordInvestment(2, investor1.address, ethers.parseEther("1000"));

      await expect(invoicePool.recordDisbursement(2)).to.be.revertedWith("Pool not filled");
    });

    it("Should not allow non-operator to disburse", async function () {
      await expect(invoicePool.connect(investor1).recordDisbursement(1)).to.be.reverted;
    });
  });

  describe("Repayment Recording", function () {
    beforeEach(async function () {
      await mintAndVerifyInvoice();
      await invoicePool.createPool(1);
      await invoicePool.recordInvestment(1, investor1.address, ethers.parseEther("4000"));
      await invoicePool.recordInvestment(1, investor2.address, ethers.parseEther("4000"));
      await invoicePool.recordDisbursement(1);
    });

    it("Should record repayment and investor returns", async function () {
      const totalRepayment = ethers.parseEther("10000");
      const investor1Return = ethers.parseEther("4900"); // Principal + interest
      const investor2Return = ethers.parseEther("4900");

      await invoicePool.recordRepayment(
        1,
        totalRepayment,
        [investor1Return, investor2Return]
      );

      const investments = await invoicePool.getPoolInvestments(1);
      expect(investments[0].actualReturn).to.equal(investor1Return);
      expect(investments[1].actualReturn).to.equal(investor2Return);
      expect(investments[0].claimed).to.be.true;
      expect(investments[1].claimed).to.be.true;
    });

    it("Should close pool after repayment", async function () {
      await invoicePool.recordRepayment(
        1,
        ethers.parseEther("10000"),
        [ethers.parseEther("4900"), ethers.parseEther("4900")]
      );

      const pool = await invoicePool.getPool(1);
      expect(pool.status).to.equal(3); // Closed
    });

    it("Should emit correct events on repayment", async function () {
      await expect(
        invoicePool.recordRepayment(
          1,
          ethers.parseEther("10000"),
          [ethers.parseEther("4900"), ethers.parseEther("4900")]
        )
      ).to.emit(invoicePool, "RepaymentRecorded")
        .and.to.emit(invoicePool, "PoolClosed");
    });

    it("Should reject invalid returns array length", async function () {
      await expect(
        invoicePool.recordRepayment(1, ethers.parseEther("10000"), [ethers.parseEther("4900")])
      ).to.be.revertedWith("Invalid returns array length");
    });

    it("Should not allow repayment on non-disbursed pool", async function () {
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
      await invoiceNFT.verifyShipment(2);
      await invoicePool.createPool(2);
      await invoicePool.recordInvestment(2, investor1.address, ethers.parseEther("8000"));

      await expect(
        invoicePool.recordRepayment(2, ethers.parseEther("10000"), [ethers.parseEther("4900")])
      ).to.be.revertedWith("Pool not disbursed");
    });

    it("Should not allow non-operator to process repayment", async function () {
      await expect(
        invoicePool.connect(investor1).recordRepayment(
          1,
          ethers.parseEther("10000"),
          [ethers.parseEther("4900"), ethers.parseEther("4900")]
        )
      ).to.be.reverted;
    });
  });

  describe("Admin Functions", function () {
    it("Should update platform fee", async function () {
      await invoicePool.setPlatformFee(300); // 3%
      expect(await invoicePool.platformFeeBps()).to.equal(300);
    });

    it("Should not allow fee above 10%", async function () {
      await expect(invoicePool.setPlatformFee(1001)).to.be.revertedWith("Fee too high");
    });

    it("Should pause and unpause", async function () {
      await mintAndVerifyInvoice();
      await invoicePool.createPool(1);

      await invoicePool.pause();
      await expect(
        invoicePool.recordInvestment(1, investor1.address, ethers.parseEther("1000"))
      ).to.be.reverted;

      await invoicePool.unpause();

      // Should work again after unpause
      await invoicePool.recordInvestment(1, investor1.address, ethers.parseEther("1000"));
      const pool = await invoicePool.getPool(1);
      expect(pool.fundedAmount).to.equal(ethers.parseEther("1000"));
    });

    it("Should update platform wallet", async function () {
      await invoicePool.setPlatformWallet(investor1.address);
      expect(await invoicePool.platformWallet()).to.equal(investor1.address);
    });

    it("Should not allow zero address for platform wallet", async function () {
      await expect(
        invoicePool.setPlatformWallet(ethers.ZeroAddress)
      ).to.be.revertedWith("Invalid address");
    });

    it("Should not allow non-admin to set platform fee", async function () {
      await expect(invoicePool.connect(investor1).setPlatformFee(300)).to.be.reverted;
    });
  });

  describe("Pool Queries", function () {
    it("Should return zero remaining capacity for non-Open pool", async function () {
      await mintAndVerifyInvoice();
      await invoicePool.createPool(1);
      await invoicePool.recordInvestment(1, investor1.address, ethers.parseEther("8000"));
      await invoicePool.recordDisbursement(1);

      const remaining = await invoicePool.getRemainingCapacity(1);
      expect(remaining).to.equal(0);
    });

    it("Should return correct remaining capacity for Open pool", async function () {
      await mintAndVerifyInvoice();
      await invoicePool.createPool(1);
      await invoicePool.recordInvestment(1, investor1.address, ethers.parseEther("3000"));

      const remaining = await invoicePool.getRemainingCapacity(1);
      expect(remaining).to.equal(ethers.parseEther("5000"));
    });

    it("Should get investor pools", async function () {
      await mintAndVerifyInvoice();
      await invoicePool.createPool(1);
      await invoicePool.recordInvestment(1, investor1.address, ethers.parseEther("3000"));

      const pools = await invoicePool.getInvestorPools(investor1.address);
      expect(pools.length).to.equal(1);
    });
  });
});
