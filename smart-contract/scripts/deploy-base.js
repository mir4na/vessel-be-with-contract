const hre = require("hardhat");
const fs = require("fs");

async function main() {
  const [deployer] = await hre.ethers.getSigners();

  console.log("========================================");
  console.log("VESSEL Smart Contract Deployment - Base");
  console.log("========================================");
  console.log("Network:", hre.network.name);
  console.log("Deployer:", deployer.address);
  console.log("Balance:", hre.ethers.formatEther(await deployer.provider.getBalance(deployer.address)), "ETH");
  console.log("========================================\n");

  let idrxTokenAddress = process.env.IDRX_TOKEN_CONTRACT_ADDRESS;

  // For testnet, deploy MockIDRX if no address provided
  if (!idrxTokenAddress || idrxTokenAddress === "0x0000000000000000000000000000000000000000") {
    console.log("1. Deploying MockIDRX Token (testnet only)...");
    const MockIDRX = await hre.ethers.getContractFactory("MockIDRX");
    const mockIDRX = await MockIDRX.deploy();
    await mockIDRX.waitForDeployment();
    idrxTokenAddress = await mockIDRX.getAddress();
    console.log("   MockIDRX deployed to:", idrxTokenAddress);
    console.log("   Initial supply: 1,000,000,000 IDRX");
  } else {
    console.log("1. Using existing IDRX token:", idrxTokenAddress);
  }

  // Deploy InvoiceNFT
  console.log("\n2. Deploying InvoiceNFT...");
  const InvoiceNFT = await hre.ethers.getContractFactory("InvoiceNFT");
  const invoiceNFT = await InvoiceNFT.deploy();
  await invoiceNFT.waitForDeployment();
  const invoiceNFTAddress = await invoiceNFT.getAddress();
  console.log("   InvoiceNFT deployed to:", invoiceNFTAddress);

  // Deploy InvoicePool
  console.log("\n3. Deploying InvoicePool...");
  const InvoicePool = await hre.ethers.getContractFactory("InvoicePool");
  const invoicePool = await InvoicePool.deploy(
    invoiceNFTAddress,
    deployer.address, // platform wallet
    idrxTokenAddress
  );
  await invoicePool.waitForDeployment();
  const invoicePoolAddress = await invoicePool.getAddress();
  console.log("   InvoicePool deployed to:", invoicePoolAddress);

  // Setup roles
  console.log("\n4. Setting up roles...");

  // Grant MINTER_ROLE to InvoicePool on InvoiceNFT
  const MINTER_ROLE = await invoiceNFT.MINTER_ROLE();
  const tx1 = await invoiceNFT.grantRole(MINTER_ROLE, invoicePoolAddress);
  await tx1.wait();
  console.log("   Granted MINTER_ROLE to InvoicePool");

  // Grant OPERATOR_ROLE to deployer on InvoicePool
  const OPERATOR_ROLE = await invoicePool.OPERATOR_ROLE();
  const tx2 = await invoicePool.grantRole(OPERATOR_ROLE, deployer.address);
  await tx2.wait();
  console.log("   Granted OPERATOR_ROLE to deployer");

  // Summary
  console.log("\n========================================");
  console.log("Deployment Complete!");
  console.log("========================================");
  console.log("Network:      ", hre.network.name);
  console.log("Chain ID:     ", (await deployer.provider.getNetwork()).chainId.toString());
  console.log("========================================");
  console.log("Contract Addresses:");
  console.log("----------------------------------------");
  console.log("IDRX Token:   ", idrxTokenAddress);
  console.log("InvoiceNFT:   ", invoiceNFTAddress);
  console.log("InvoicePool:  ", invoicePoolAddress);
  console.log("========================================");

  // Determine explorer URL
  const chainId = (await deployer.provider.getNetwork()).chainId.toString();
  let explorerUrl = "https://sepolia.basescan.org";
  if (chainId === "8453") {
    explorerUrl = "https://basescan.org";
  }

  console.log("\nVerify contracts on BaseScan:");
  console.log(`${explorerUrl}/address/${idrxTokenAddress}#code`);
  console.log(`${explorerUrl}/address/${invoiceNFTAddress}#code`);
  console.log(`${explorerUrl}/address/${invoicePoolAddress}#code`);

  // Save deployment info
  const deploymentInfo = {
    network: hre.network.name,
    chainId: chainId,
    deployer: deployer.address,
    deployedAt: new Date().toISOString(),
    contracts: {
      MockIDRX: idrxTokenAddress,
      InvoiceNFT: invoiceNFTAddress,
      InvoicePool: invoicePoolAddress,
    },
    explorerUrl: explorerUrl,
  };

  const filename = `./deployments-${hre.network.name}.json`;
  fs.writeFileSync(filename, JSON.stringify(deploymentInfo, null, 2));
  console.log(`\nDeployment info saved to ${filename}`);

  // Print env variables to copy
  console.log("\n========================================");
  console.log("Add these to your .env file:");
  console.log("========================================");
  console.log(`IDRX_TOKEN_CONTRACT_ADDRESS=${idrxTokenAddress}`);
  console.log(`INVOICE_NFT_CONTRACT_ADDRESS=${invoiceNFTAddress}`);
  console.log(`INVOICE_POOL_CONTRACT_ADDRESS=${invoicePoolAddress}`);
  if (chainId === "84532") {
    console.log(`BLOCKCHAIN_RPC_URL=https://sepolia.base.org`);
    console.log(`CHAIN_ID=84532`);
    console.log(`BLOCK_EXPLORER_URL=https://sepolia.basescan.org`);
  } else {
    console.log(`BLOCKCHAIN_RPC_URL=https://mainnet.base.org`);
    console.log(`CHAIN_ID=8453`);
    console.log(`BLOCK_EXPLORER_URL=https://basescan.org`);
  }
  console.log("========================================");
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error("Deployment failed:", error);
    process.exit(1);
  });
