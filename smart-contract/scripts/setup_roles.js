const hre = require("hardhat");

async function main() {
    const [deployer] = await hre.ethers.getSigners();
    console.log("Setting up roles with account:", deployer.address);

    // Addresses from .env or manual input (since .env might not be loaded by hardhat automatically if not using dotenv in config)
    // For safety, I'll read from process.env if dotenv is set up, or hardcode/pass them.
    // Hardhat config usually loads dotenv.

    const INVOICE_NFT_ADDRESS = "0x0e4b822f012cDab1e053BA4a11F50A0A137BE445";
    const INVOICE_POOL_ADDRESS = "0x86292A6B7f86445ED93c5C2f89279315Cab7c179";

    console.log("InvoiceNFT:", INVOICE_NFT_ADDRESS);
    console.log("InvoicePool:", INVOICE_POOL_ADDRESS);

    // Attach contracts
    const InvoiceNFT = await hre.ethers.getContractFactory("InvoiceNFT");
    const invoiceNFT = InvoiceNFT.attach(INVOICE_NFT_ADDRESS);

    const InvoicePool = await hre.ethers.getContractFactory("InvoicePool");
    const invoicePool = InvoicePool.attach(INVOICE_POOL_ADDRESS);

    // 1. Grant MINTER_ROLE to InvoicePool
    const MINTER_ROLE = await invoiceNFT.MINTER_ROLE();
    const hasMinter = await invoiceNFT.hasRole(MINTER_ROLE, INVOICE_POOL_ADDRESS);
    if (!hasMinter) {
        console.log("Granting MINTER_ROLE to InvoicePool...");
        const tx = await invoiceNFT.grantRole(MINTER_ROLE, INVOICE_POOL_ADDRESS);
        await tx.wait();
        console.log("Done.");
    } else {
        console.log("InvoicePool already has MINTER_ROLE");
    }

    // 2. Grant OPERATOR_ROLE to deployer
    const OPERATOR_ROLE = await invoicePool.OPERATOR_ROLE();
    const hasOperator = await invoicePool.hasRole(OPERATOR_ROLE, deployer.address);
    if (!hasOperator) {
        console.log("Granting OPERATOR_ROLE to deployer...");
        const tx = await invoicePool.grantRole(OPERATOR_ROLE, deployer.address);
        await tx.wait();
        console.log("Done.");
    } else {
        console.log("Deployer already has OPERATOR_ROLE");
    }
}

main()
    .then(() => process.exit(0))
    .catch((error) => {
        console.error(error);
        process.exit(1);
    });
