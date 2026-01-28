use ethers::{
    contract::abigen,
    prelude::*,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, H256, U256},
};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::repository::{FundingRepository, InvoiceRepository};

use super::PinataService;

// Generate ERC20 contract bindings for IDRX token
abigen!(
    IERC20,
    r#"[
        function name() external view returns (string)
        function symbol() external view returns (string)
        function decimals() external view returns (uint8)
        function totalSupply() external view returns (uint256)
        function balanceOf(address account) external view returns (uint256)
        function transfer(address to, uint256 amount) external returns (bool)
        function allowance(address owner, address spender) external view returns (uint256)
        function approve(address spender, uint256 amount) external returns (bool)
        function transferFrom(address from, address to, uint256 amount) external returns (bool)
        event Transfer(address indexed from, address indexed to, uint256 value)
        event Approval(address indexed owner, address indexed spender, uint256 value)
    ]"#
);

// Generate InvoiceNFT contract bindings
abigen!(
    InvoiceNFT,
    r#"[
        function mintInvoice(address to, string memory invoiceNumber, uint256 amount, uint256 advanceAmount, uint256 interestRate, uint256 issueDate, uint256 dueDate, string memory buyerCountry, string memory documentHash, string memory uri) external returns (uint256)
        function getTokenIdByInvoiceNumber(string memory invoiceNumber) external view returns (uint256)
    ]"#
);

// Generate InvoicePool contract bindings
abigen!(
    InvoicePool,
    r#"[
        function createPool(uint256 tokenId) external
        function recordInvestment(uint256 tokenId, address investor, uint256 amount) external
        function recordRepayment(uint256 tokenId, uint256 totalAmount, uint256[] calldata investorReturns) external
        function closePoolEarly(uint256 tokenId) external
    ]"#
);

// EIP-1271 Interface
abigen!(
    IERC1271,
    r#"[
        function isValidSignature(bytes32 _hash, bytes memory _signature) public view returns (bytes4)
    ]"#
);

// EIP-6492 UniversalSigValidator
abigen!(
    UniversalSigValidator,
    r#"[
        function isValidSig(address _signer, bytes32 _hash, bytes memory _signature) external returns (bool)
    ]"#
);

/// Represents a verified on-chain IDRX transfer
#[derive(Debug, Clone, serde::Serialize)]
pub struct VerifiedTransfer {
    pub tx_hash: String,
    pub from: String,
    pub to: String,
    pub amount: Decimal,
    pub block_number: u64,
    pub confirmed: bool,
    pub explorer_url: String,
}

/// Transaction type for on-chain records
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OnChainTxType {
    Investment,
    Disbursement,
    Repayment,
    InvestorReturn,
    PlatformFee,
}

pub struct BlockchainService {
    config: Arc<Config>,
    provider: Provider<Http>,
    wallet: Option<LocalWallet>,
    invoice_repo: Arc<InvoiceRepository>,
    funding_repo: Arc<FundingRepository>,
    pinata_service: Arc<PinataService>,
    idrx_decimals: u8,
}

impl BlockchainService {
    pub async fn new(
        config: Arc<Config>,
        invoice_repo: Arc<InvoiceRepository>,
        funding_repo: Arc<FundingRepository>,
        pinata_service: Arc<PinataService>,
    ) -> AppResult<Self> {
        let provider = Provider::<Http>::try_from(&config.blockchain_rpc_url)
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;

        let wallet = if !config.private_key.is_empty() {
            let wallet: LocalWallet = config
                .private_key
                .parse()
                .map_err(|e: WalletError| AppError::BlockchainError(e.to_string()))?;
            Some(wallet.with_chain_id(config.chain_id))
        } else {
            tracing::warn!("Private key not configured, blockchain operations will be limited");
            None
        };

        // IDRX uses 2 decimals (like IDR)
        let idrx_decimals = 2u8;

        Ok(Self {
            config,
            provider,
            wallet,
            invoice_repo,
            funding_repo,
            pinata_service,
            idrx_decimals,
        })
    }

    // ==================== IDRX Token Methods ====================

    /// Get IDRX token contract instance
    fn get_idrx_contract(&self) -> AppResult<IERC20<Provider<Http>>> {
        let contract_addr: Address =
            self.config.idrx_token_contract_addr.parse().map_err(|_| {
                AppError::BlockchainError("Invalid IDRX contract address".to_string())
            })?;

        Ok(IERC20::new(contract_addr, Arc::new(self.provider.clone())))
    }

    /// Get IDRX balance for an address
    pub async fn get_idrx_balance(&self, address: &str) -> AppResult<Decimal> {
        let addr: Address = address
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid address".to_string()))?;

        let contract = self.get_idrx_contract()?;
        let balance: U256 =
            contract.balance_of(addr).call().await.map_err(|e| {
                AppError::BlockchainError(format!("Failed to get IDRX balance: {}", e))
            })?;

        // Convert from token units to Decimal (IDRX has 2 decimals)
        let balance_f64 = balance.as_u128() as f64 / 10f64.powi(self.idrx_decimals as i32);
        Ok(Decimal::from_f64_retain(balance_f64).unwrap_or(Decimal::ZERO))
    }

    /// Get platform wallet IDRX balance (escrow balance)
    pub async fn get_platform_idrx_balance(&self) -> AppResult<Decimal> {
        self.get_idrx_balance(&self.config.platform_wallet_address)
            .await
    }

    /// Convert Decimal amount to token units (U256)
    fn to_token_units(&self, amount: Decimal) -> U256 {
        let multiplier = 10u128.pow(self.idrx_decimals as u32);
        let amount_u128 = (amount.to_f64().unwrap_or(0.0) * multiplier as f64) as u128;
        U256::from(amount_u128)
    }

    /// Verify signature using ERC-1271 (for Smart Contract Wallets)
    pub async fn verify_signature_erc1271(
        &self,
        wallet_address: &str,
        message_hash: [u8; 32],
        signature: Vec<u8>,
    ) -> AppResult<bool> {
        let addr: Address = wallet_address
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid wallet address".to_string()))?;
        
        // Check if address has code (Deployed Smart Wallet)
        let code = self.provider.get_code(addr, None).await.map_err(|e| {
            AppError::BlockchainError(format!("Failed to get code: {}", e))
        })?;

        if !code.is_empty() {
             // 0x1626ba7e is the bytes4 magic value for isValidSignature
            let magic_value = [0x16, 0x26, 0xba, 0x7e];
            let contract = IERC1271::new(addr, Arc::new(self.provider.clone()));
            
            // Try standard ERC-1271 first for deployed contracts
            let result = contract
                .is_valid_signature(message_hash, signature.clone().into())
                .call()
                .await;

             match result {
                Ok(val) => {
                    if val == magic_value {
                        return Ok(true);
                    }
                },
                Err(e) => {
                     tracing::warn!("Standard ERC-1271 failed for deployed contract: {}", e);
                }
            }
        }
        
        // Fallback to Universal Signature Validator (EIP-6492)
        // This handles undeployed contracts (counterfactual) and also retries 1271 securely
        self.verify_signature_universal(addr, message_hash, signature).await
    }
    
    /// Use EIP-6492 Universal Signature Validator
    /// Contract: 0x6492c034cc609e99298b3097c29bc906df0c0522 (Base Mainnet & Sepolia)
    /// Use EIP-6492 Universal Signature Validator
    /// Contract: 0x6492c034cc609e99298b3097c29bc906df0c0522 (Base Mainnet & Sepolia)
    async fn verify_signature_universal(
        &self,
        signer: Address,
        hash: [u8; 32],
        signature: Vec<u8>,
    ) -> AppResult<bool> {
        let validator_addr: Address = "0x6492c034cc609e99298b3097c29bc906df0c0522".parse().unwrap();
        
        // UniversalSigValidator Runtime Bytecode (fetched from Base Mainnet)
        // This allows us to use state overrides if the contract is missing on the current chain
        let validator_bytecode = "0x608060405234801561000f575f5ffd5b506004361061003f575f3560e01c806376be4cea146100435780638f0684301461007357806398ef1ed8146100a3575b5f5ffd5b61005d600480360381019061005891906108e1565b6100d3565b60405161006a9190610986565b60405180910390f35b61008d6004803603810190610088919061099f565b6105fe565b60405161009a9190610986565b60405180910390f35b6100bd60048036038101906100b8919061099f565b61068d565b6040516100ca9190610986565b60405180910390f35b5f5f8773ffffffffffffffffffffffffffffffffffffffff163b905060605f7f64926492649264926492649264926492649264926492649264926492649264925f1b888860208b8b90506101279190610a46565b908b8b90509261013993929190610a81565b906101449190610ad1565b1490508015610250575f606089895f9060208d8d90506101649190610a46565b9261017193929190610a81565b81019061017e9190610ca2565b8096508193508294505050505f8514806101955750865b15610249575f5f8373ffffffffffffffffffffffffffffffffffffffff16836040516101c19190610d7c565b5f604051808303815f865af19150503d805f81146101fa576040519150601f19603f3d011682016040523d82523d5f602084013e6101ff565b606091505b50915091508161024657806040517f9d0d6e2d00000000000000000000000000000000000000000000000000000000815260040161023d9190610dda565b60405180910390fd5b50505b5050610297565b87878080601f0160208091040260200160405190810160405280939291908181526020018383808284375f81840152601f19601f8201169050808301925050505050505091505b80806102a257505f83115b1561046c578973ffffffffffffffffffffffffffffffffffffffff16631626ba7e8a846040518363ffffffff1660e01b81526004016102e2929190610e09565b602060405180830381865afa92505050801561031c57506040513d601f19601f820116820180604052508101906103199190610e8c565b60015b6103b9573d805f811461034a576040519150601f19603f3d011682016040523d82523d5f602084013e61034f565b606091505b508515801561035d57505f84115b1561037c576103718b8b8b8b8b60016100d3565b9450505050506105f4565b806040517f6f2a95990000000000000000000000000000000000000000000000000000000081526004016103b09190610dda565b60405180910390fd5b5f631626ba7e60e01b7bffffffffffffffffffffffffffffffffffffffffffffffffffffffff1916827bffffffffffffffffffffffffffffffffffffffffffffffffffffffff191614905080158015610410575086155b801561041b57505f85115b1561043b5761042f8c8c8c8c8c60016100d3565b955050505050506105f4565b5f851480156104475750825b8015610451575087155b1561045f57805f526001601ffd5b80955050505050506105f4565b604188889050146104b2576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016104a990610f37565b60405180910390fd5b5f88885f906020926104c693929190610a81565b906104d19190610ad1565b90505f89896020906040926104e893929190610a81565b906104f39190610ad1565b90505f8a8a604081811061050a57610509610f55565b5b9050013560f81c60f81b60f81c9050601b8160ff16141580156105315750601c8160ff1614155b15610571576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161056890610ff2565b60405180910390fd5b8c73ffffffffffffffffffffffffffffffffffffffff1660018d8386866040515f81526020016040526040516105aa949392919061102b565b6020604051602081039080840390855afa1580156105ca573d5f5f3e3d5ffd5b5050506020604051035173ffffffffffffffffffffffffffffffffffffffff161496505050505050505b9695505050505050565b5f3073ffffffffffffffffffffffffffffffffffffffff166376be4cea8686868660015f6040518763ffffffff1660e01b8152600401610643969594939291906110a9565b6020604051808303815f875af115801561065f573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906106839190611117565b9050949350505050565b5f3073ffffffffffffffffffffffffffffffffffffffff166376be4cea868686865f5f6040518763ffffffff1660e01b81526004016106d1969594939291906110a9565b6020604051808303815f875af192505050801561070c57506040513d601f19601f820116820180604052508101906107099190611117565b60015b6107a0573d805f811461073a576040519150601f19603f3d011682016040523d82523d5f602084013e61073f565b606091505b505f815190506001810361079c57600160f81b825f8151811061076557610764610f55565b5b602001015160f81c60f81b7effffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff191614925050506107a5565b8082fd5b809150505b949350505050565b5f604051905090565b5f5ffd5b5f5ffd5b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b5f6107e7826107be565b9050919050565b6107f7816107dd565b8114610801575f5ffd5b50565b5f81359050610812816107ee565b92915050565b5f819050919050565b61082a81610818565b8114610834575f5ffd5b50565b5f8135905061084581610821565b92915050565b5f5ffd5b5f5ffd5b5f5ffd5b5f5f83601f84011261086c5761086b61084b565b5b8235905067ffffffffffffffff8111156108895761088861084f565b5b6020830191508360018202830111156108a5576108a4610853565b5b9250929050565b5f8115159050919050565b6108c0816108ac565b81146108ca575f5ffd5b50565b5f813590506108db816108b7565b92915050565b5f5f5f5f5f5f60a087890312156108fb576108fa6107b6565b5b5f61090889828a01610804565b965050602061091989828a01610837565b955050604087013567ffffffffffffffff81111561093a576109396107ba565b5b61094689828a01610857565b9450945050606061095989828a016108cd565b925050608061096a89828a016108cd565b9150509295509295509295565b610980816108ac565b82525050565b5f6020820190506109995f830184610977565b92915050565b5f5f5f5f606085870312156109b7576109b66107b6565b5b5f6109c487828801610804565b94505060206109d587828801610837565b935050604085013567ffffffffffffffff8111156109f6576109f56107ba565b5b610a0287828801610857565b925092505092959194509250565b5f819050919050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601160045260245ffd5b5f610a5082610a10565b9150610a5b83610a10565b9250828203905081811115610a7357610a72610a19565b5b92915050565b5f5ffd5b5f5ffd5b5f5f85851115610a9457610a93610a79565b5b83861115610aa557610aa4610a7d565b5b6001850283019150848603905094509492505050565b5f82905092915050565b5f82821b905092915050565b5f610adc8383610abb565b82610ae78135610818565b92506020821015610b2757610b227fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff83602003600802610ac5565b831692505b505092915050565b5f610b39826107be565b9050919050565b610b4981610b2f565b8114610b53575f5ffd5b50565b5f81359050610b6481610b40565b92915050565b5f5ffd5b5f601f19601f8301169050919050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52604160045260245ffd5b610bb482610b6e565b810181811067ffffffffffffffff82111715610bd357610bd2610b7e565b5b80604052505050565b5f610be56107ad565b9050610bf18282610bab565b919050565b5f67ffffffffffffffff821115610c1057610c0f610b7e565b5b610c1982610b6e565b9050602081019050919050565b828183375f83830152505050565b5f610c46610c4184610bf6565b610bdc565b905082815260208101848484011115610c6257610c61610b6a565b5b610c6d848285610c26565b509392505050565b5f82601f830112610c8957610c8861084b565b5b8135610c99848260208601610c34565b91505092915050565b5f5f5f60608486031215610cb957610cb86107b6565b5b5f610cc686828701610b56565b935050602084013567ffffffffffffffff811115610ce757610ce66107ba565b5b610cf386828701610c75565b925050604084013567ffffffffffffffff811115610d1457610d136107ba565b5b610d2086828701610c75565b9150509250925092565b5f81519050919050565b5f81905092915050565b8281835e5f83830152505050565b5f610d5682610d2a565b610d608185610d34565b9350610d70818560208601610d3e565b80840191505092915050565b5f610d878284610d4c565b915081905092915050565b5f82825260208201905092915050565b5f610dac82610d2a565b610db68185610d92565b9350610dc6818560208601610d3e565b610dcf81610b6e565b840191505092915050565b5f6020820190508181035f830152610df28184610da2565b905092915050565b610e0381610818565b82525050565b5f604082019050610e1c5f830185610dfa565b8181036020830152610e2e8184610da2565b90509392505050565b5f7fffffffff0000000000000000000000000000000000000000000000000000000082169050919050565b610e6b81610e37565b8114610e75575f5ffd5b50565b5f81519050610e8681610e62565b92915050565b5f60208284031215610ea157610ea06107b6565b5b5f610eae84828501610e78565b91505092915050565b5f82825260208201905092915050565b7f5369676e617475726556616c696461746f72237265636f7665725369676e65725f8201527f3a20696e76616c6964207369676e6174757265206c656e677468000000000000602082015250565b5f610f21603a83610eb7565b9150610f2c82610ec7565b604082019050919050565b5f6020820190508181035f830152610f4e81610f15565b9050919050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52603260045260245ffd5b7f5369676e617475726556616c696461746f723a20696e76616c6964207369676e5f8201527f617475726520762076616c756500000000000000000000000000000000000000602082015250565b5f610fdc602d83610eb7565b9150610fe782610f82565b604082019050919050565b5f6020820190508181035f83015261100981610fd0565b9050919050565b5f60ff82169050919050565b61102581611010565b82525050565b5f60808201905061103e5f830187610dfa565b61104b602083018661101c565b6110586040830185610dfa565b6110656060830184610dfa565b95945050505050565b611077816107dd565b82525050565b5f6110888385610d92565b9350611095838584610c26565b61109e83610b6e565b840190509392505050565b5f60a0820190506110bc5f83018961106e565b6110c96020830188610dfa565b81810360408301526110dc81868861107d565b90506110eb6060830185610977565b6110f86080830184610977565b979650505050505050565b5f81519050611111816108b7565b92915050565b5f6020828403121561112c5761112b6107b6565b5b5f61113984828501611103565b9150509291505056fea2646970667358221220a097e3b3de576882cc80ec9fc7e5e58495b422f643739b61de8d128d51ee11ee64736f6c634300081c0033";
        
        let contract = UniversalSigValidator::new(validator_addr, Arc::new(self.provider.clone()));
        
        // Encode the transaction data
        let calldata = contract
            .is_valid_sig(signer, hash, signature.into())
            .calldata()
            .ok_or_else(|| AppError::BlockchainError("Failed to encode calldata".to_string()))?;

        // Construct the raw JSON-RPC request for eth_call with state overrides
        // Params: [ { to, data }, "latest", { address: { code } } ]
        
        let tx_obj = serde_json::json!({
            "to": validator_addr,
            "data": calldata,
        });
        
        // State override object: address -> { code: bytecode }
        let state_overrides = serde_json::json!({
            format!("{:?}", validator_addr): {
                "code": validator_bytecode
            }
        });
        
        let params = (tx_obj, "latest", state_overrides);
        
        let result: Result<ethers::types::Bytes, _> = self.provider
            .request("eth_call", params)
            .await;
            
        match result {
            Ok(bytes) => {
                // Decode bool result (first 32 bytes)
                if bytes.len() >= 32 {
                     let is_valid = bytes[31] != 0; // check last byte of 32-byte word
                     return Ok(is_valid);
                }
                Ok(false)
            },
            Err(e) => {
                 tracing::error!("Universal Sig Validator (State Override) failed: {:?}", e);
                 Ok(false)
            }
        }
    }

    /// Prepare message hash for verification (matches EIP-191 Personal Sign)
    pub fn hash_message(&self, message: &str) -> [u8; 32] {
        ethers::utils::hash_message(message).into()
    }

    /// Verify an IDRX transfer transaction
    /// Returns details if the transfer is valid and matches expected parameters
    pub async fn verify_idrx_transfer(
        &self,
        tx_hash: &str,
        expected_to: &str,
        expected_amount: Decimal,
    ) -> AppResult<VerifiedTransfer> {
        if self.config.skip_blockchain_verification {
            tracing::info!("SKIPPING blockchain verification (Test Mode)");
            return Ok(VerifiedTransfer {
                tx_hash: tx_hash.to_string(),
                from: "0xTestUser".to_string(),
                to: expected_to.to_string(),
                amount: expected_amount,
                block_number: 12345,
                confirmed: true,
                explorer_url: "http://test.com".to_string(),
            });
        }

        let hash: H256 = tx_hash
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid transaction hash".to_string()))?;

        let expected_to_addr: Address = expected_to
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid recipient address".to_string()))?;

        // Get the transaction receipt
        let receipt = self
            .provider
            .get_transaction_receipt(hash)
            .await
            .map_err(|e| {
                AppError::BlockchainError(format!("Failed to get tx receipt: {}", e))
            })?
            .ok_or_else(|| {
                AppError::BlockchainError("Transaction not found or not confirmed".to_string())
            })?;

        // Check transaction succeeded
        let status = receipt
            .status
            .ok_or_else(|| AppError::BlockchainError("Transaction status unknown".to_string()))?;
        if status.as_u64() != 1 {
            return Err(AppError::BlockchainError("Transaction failed".to_string()));
        }

        // Parse Transfer events from logs
        let contract_addr: Address =
            self.config.idrx_token_contract_addr.parse().map_err(|_| {
                AppError::BlockchainError("Invalid IDRX contract address".to_string())
            })?;

        // Transfer event signature: Transfer(address,address,uint256)
        let transfer_topic = H256::from_slice(&ethers::utils::keccak256(
            "Transfer(address,address,uint256)",
        ));

        let mut verified_from = String::new();
        let mut verified_amount = Decimal::ZERO;
        let mut found_transfer = false;

        for log in receipt.logs.iter() {
            // Check if log is from IDRX contract and is a Transfer event
            if log.address == contract_addr
                && !log.topics.is_empty()
                && log.topics[0] == transfer_topic
            {
                // topics[1] = from, topics[2] = to (both padded to 32 bytes)
                if log.topics.len() >= 3 {
                    let to_addr = Address::from_slice(&log.topics[2].as_bytes()[12..32]);

                    if to_addr == expected_to_addr {
                        let from_addr = Address::from_slice(&log.topics[1].as_bytes()[12..32]);
                        verified_from = format!("{:?}", from_addr);

                        // Amount is in data field
                        let amount_u256 = U256::from_big_endian(&log.data);
                        let amount_f64 =
                            amount_u256.as_u128() as f64 / 10f64.powi(self.idrx_decimals as i32);
                        verified_amount =
                            Decimal::from_f64_retain(amount_f64).unwrap_or(Decimal::ZERO);
                        found_transfer = true;
                        break;
                    }
                }
            }
        }

        if !found_transfer {
            return Err(AppError::BlockchainError(
                "No matching IDRX transfer found to expected recipient".to_string(),
            ));
        }

        // Verify amount (allow small rounding difference)
        let diff = (verified_amount - expected_amount).abs();
        if diff > Decimal::from_f64_retain(0.01).unwrap() {
            return Err(AppError::BlockchainError(format!(
                "Transfer amount mismatch: expected {}, got {}",
                expected_amount, verified_amount
            )));
        }

        let block_number = receipt.block_number.map(|n| n.as_u64()).unwrap_or(0);

        Ok(VerifiedTransfer {
            tx_hash: tx_hash.to_string(),
            from: verified_from,
            to: expected_to.to_string(),
            amount: verified_amount,
            block_number,
            confirmed: true,
            explorer_url: self.get_explorer_url(tx_hash),
        })
    }

    /// Verify investment transfer - investor sends IDRX to platform wallet
    pub async fn verify_investment_transfer(
        &self,
        tx_hash: &str,
        expected_amount: Decimal,
    ) -> AppResult<VerifiedTransfer> {
        self.verify_idrx_transfer(
            tx_hash,
            &self.config.platform_wallet_address,
            expected_amount,
        )
        .await
    }

    /// Transfer IDRX from platform wallet to a recipient
    /// Used for disbursements to exporters and returns to investors
    pub async fn transfer_idrx(
        &self,
        to_address: &str,
        amount: Decimal,
        tx_type: OnChainTxType,
    ) -> AppResult<String> {
        if self.config.skip_blockchain_verification {
            tracing::info!("SKIPPING blockchain transfer logic (Test Mode)");
            return Ok(format!("0xTestTransferHash_{}", Uuid::new_v4()));
        }

        let wallet = self.wallet.as_ref().ok_or_else(|| {
            AppError::BlockchainError("Platform wallet not configured".to_string())
        })?;

        let to_addr: Address = to_address
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid recipient address".to_string()))?;

        let contract_addr: Address =
            self.config.idrx_token_contract_addr.parse().map_err(|_| {
                AppError::BlockchainError("Invalid IDRX contract address".to_string())
            })?;

        let client = SignerMiddleware::new(self.provider.clone(), wallet.clone());
        let contract = IERC20::new(contract_addr, Arc::new(client));

        let amount_units = self.to_token_units(amount);

        tracing::info!(
            "Transferring {} IDRX to {} for {:?}",
            amount,
            to_address,
            tx_type
        );

        let tx = contract.transfer(to_addr, amount_units);
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| AppError::BlockchainError(format!("Transfer failed: {}", e)))?;

        let tx_hash = format!("{:?}", pending_tx.tx_hash());

        // Wait for confirmation
        let receipt = pending_tx
            .await
            .map_err(|e| AppError::BlockchainError(format!("Transaction failed: {}", e)))?
            .ok_or_else(|| AppError::BlockchainError("Transaction dropped".to_string()))?;

        if receipt.status.map(|s| s.as_u64()) != Some(1) {
            return Err(AppError::BlockchainError(
                "Transfer transaction failed".to_string(),
            ));
        }

        tracing::info!(
            "IDRX transfer completed: {} - {} IDRX to {} (block: {:?})",
            tx_hash,
            amount,
            to_address,
            receipt.block_number
        );

        Ok(tx_hash)
    }

    /// Disburse funds to exporter (transfer IDRX from platform to exporter)
    pub async fn disburse_to_exporter(
        &self,
        exporter_wallet: &str,
        amount: Decimal,
        pool_id: Uuid,
    ) -> AppResult<String> {
        tracing::info!(
            "Disbursing {} IDRX to exporter for pool {}",
            amount,
            pool_id
        );
        self.transfer_idrx(exporter_wallet, amount, OnChainTxType::Disbursement)
            .await
    }

    /// Return funds to investor (transfer IDRX from platform to investor)
    pub async fn return_to_investor(
        &self,
        investor_wallet: &str,
        amount: Decimal,
        pool_id: Uuid,
    ) -> AppResult<String> {
        tracing::info!("Returning {} IDRX to investor for pool {}", amount, pool_id);
        self.transfer_idrx(investor_wallet, amount, OnChainTxType::InvestorReturn)
            .await
    }

    /// Get all IDRX transfers for an address (for transparency/audit)
    pub async fn get_transfer_history(
        &self,
        address: &str,
        from_block: Option<u64>,
    ) -> AppResult<Vec<serde_json::Value>> {
        let addr: Address = address
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid address".to_string()))?;

        let contract_addr: Address =
            self.config.idrx_token_contract_addr.parse().map_err(|_| {
                AppError::BlockchainError("Invalid IDRX contract address".to_string())
            })?;

        let transfer_topic = H256::from_slice(&ethers::utils::keccak256(
            "Transfer(address,address,uint256)",
        ));

        // Pad address to 32 bytes for topic filter
        let addr_topic = H256::from_slice(&{
            let mut padded = [0u8; 32];
            padded[12..32].copy_from_slice(addr.as_bytes());
            padded
        });

        let from = from_block.map(U64::from).unwrap_or(U64::from(0));

        // Get incoming transfers (to = address)
        let incoming_filter = Filter::new()
            .address(contract_addr)
            .topic0(transfer_topic)
            .topic2(addr_topic)
            .from_block(from);

        // Get outgoing transfers (from = address)
        let outgoing_filter = Filter::new()
            .address(contract_addr)
            .topic0(transfer_topic)
            .topic1(addr_topic)
            .from_block(from);

        let incoming_logs = self
            .provider
            .get_logs(&incoming_filter)
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;

        let outgoing_logs = self
            .provider
            .get_logs(&outgoing_filter)
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;

        let mut transfers = Vec::new();

        for log in incoming_logs.iter().chain(outgoing_logs.iter()) {
            if log.topics.len() >= 3 {
                let from_addr = Address::from_slice(&log.topics[1].as_bytes()[12..32]);
                let to_addr = Address::from_slice(&log.topics[2].as_bytes()[12..32]);
                let amount_u256 = U256::from_big_endian(&log.data);
                let amount = amount_u256.as_u128() as f64 / 10f64.powi(self.idrx_decimals as i32);

                transfers.push(serde_json::json!({
                    "tx_hash": format!("{:?}", log.transaction_hash.unwrap_or_default()),
                    "block_number": log.block_number.map(|n| n.as_u64()),
                    "from": format!("{:?}", from_addr),
                    "to": format!("{:?}", to_addr),
                    "amount": amount,
                    "direction": if to_addr == addr { "incoming" } else { "outgoing" },
                    "explorer_url": self.get_explorer_url(&format!("{:?}", log.transaction_hash.unwrap_or_default())),
                }));
            }
        }

        Ok(transfers)
    }

    pub async fn get_chain_id(&self) -> AppResult<u64> {
        let chain_id = self
            .provider
            .get_chainid()
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;
        Ok(chain_id.as_u64())
    }

    pub async fn get_block_number(&self) -> AppResult<u64> {
        let block_number = self
            .provider
            .get_block_number()
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;
        Ok(block_number.as_u64())
    }

    pub async fn get_balance(&self, address: &str) -> AppResult<U256> {
        let addr: Address = address
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid address".to_string()))?;

        let balance = self
            .provider
            .get_balance(addr, None)
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;

        Ok(balance)
    }

    pub async fn verify_transaction(&self, tx_hash: &str) -> AppResult<bool> {
        let hash: TxHash = tx_hash
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid transaction hash".to_string()))?;

        let receipt = self
            .provider
            .get_transaction_receipt(hash)
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;

        match receipt {
            Some(r) => Ok(r.status.map(|s| s.as_u64() == 1).unwrap_or(false)),
            None => Ok(false),
        }
    }

    pub async fn get_transaction_block(&self, tx_hash: &str) -> AppResult<Option<u64>> {
        let hash: TxHash = tx_hash
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid transaction hash".to_string()))?;

        let receipt = self
            .provider
            .get_transaction_receipt(hash)
            .await
            .map_err(|e| AppError::BlockchainError(e.to_string()))?;

        Ok(receipt.and_then(|r| r.block_number).map(|n| n.as_u64()))
    }

    pub fn get_explorer_url(&self, tx_hash: &str) -> String {
        format!("{}/tx/{}", self.config.block_explorer_url, tx_hash)
    }

    pub fn get_contract_address(&self) -> &str {
        &self.config.invoice_nft_contract_addr
    }

    pub fn get_platform_wallet(&self) -> &str {
        &self.config.platform_wallet_address
    }

    // Note: Full NFT minting would require ABI bindings
    // This is a simplified version - in production, use ethers-rs contract bindings
    pub async fn mint_invoice_nft(
        &self,
        invoice: &crate::models::Invoice,
        uri: &str,
    ) -> AppResult<(i64, String, String)> {
        let contract_addr: Address =
            self.config.invoice_nft_contract_addr.parse().map_err(|_| {
                AppError::BlockchainError("Invalid InvoiceNFT contract address".to_string())
            })?;

        if self.config.skip_blockchain_verification {
            tracing::info!("SKIPPING blockchain minting (Test Mode)");
            return Ok((
                12345,
                "0xTestMintHash".to_string(),
                contract_addr.to_string(),
            ));
        }

        let wallet = self.wallet.as_ref().ok_or_else(|| {
            AppError::BlockchainError("Platform wallet not configured".to_string())
        })?;

        let client = SignerMiddleware::new(self.provider.clone(), wallet.clone());
        let contract = InvoiceNFT::new(contract_addr, Arc::new(client));

        // Prepare args
        let to_addr: Address = invoice
            .exporter_wallet_address
            .as_deref()
            .ok_or_else(|| {
                AppError::ValidationError("Exporter wallet address required".to_string())
            })?
            .parse()
            .map_err(|_| {
                AppError::ValidationError("Invalid exporter wallet address".to_string())
            })?;

        let amount_units = self.to_token_units(invoice.amount);
        let advance_amount = invoice.advance_amount.unwrap_or(invoice.amount);
        let advance_units = self.to_token_units(advance_amount);

        let interest_bps: u64 = invoice
            .interest_rate
            .map(|r| (r.to_f64().unwrap_or(0.0) * 100.0) as u64) // e.g. 10.5% -> 1050 bps
            .unwrap_or(0);

        let issue_date = U256::from(
            invoice
                .issue_date
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp(),
        );
        let due_date = U256::from(
            invoice
                .due_date
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp(),
        );

        let doc_hash = invoice.document_hash.clone().unwrap_or_default();

        tracing::info!("Minting NFT for invoice {}", invoice.invoice_number);

        let tx = contract.mint_invoice(
            to_addr,
            invoice.invoice_number.clone(),
            amount_units,
            advance_units,
            U256::from(interest_bps),
            issue_date,
            due_date,
            invoice.buyer_country.clone(),
            doc_hash,
            uri.to_string(),
        );

        let pending_tx = tx
            .send()
            .await
            .map_err(|e| AppError::BlockchainError(format!("Failed to send mint tx: {}", e)))?;

        let receipt = pending_tx
            .await
            .map_err(|e| {
                AppError::BlockchainError(format!("Failed to wait for mint receipt: {}", e))
            })?
            .ok_or_else(|| AppError::BlockchainError("Mint transaction failed".to_string()))?;

        let tx_hash = format!("{:?}", receipt.transaction_hash);

        // Find TokenId by querying contract
        let token_id_u256 = contract
            .get_token_id_by_invoice_number(invoice.invoice_number.clone())
            .call()
            .await
            .map_err(|e| AppError::BlockchainError(format!("Failed to get token ID: {}", e)))?;

        let token_id = token_id_u256.as_u64() as i64;
        let contract_address_str = self.config.invoice_nft_contract_addr.clone();

        Ok((token_id, tx_hash, contract_address_str))
    }

    pub async fn create_nft_metadata(&self, invoice_id: Uuid) -> AppResult<String> {
        let invoice = self
            .invoice_repo
            .find_by_id(invoice_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        let metadata = serde_json::json!({
            "name": format!("VESSEL Invoice #{}", invoice.invoice_number),
            "description": format!("Tokenized invoice from VESSEL platform"),
            "image": "https://vessel.io/nft-image.png",
            "external_url": format!("https://vessel.io/invoices/{}", invoice_id),
            "attributes": [
                {
                    "trait_type": "Invoice Number",
                    "value": invoice.invoice_number
                },
                {
                    "trait_type": "Amount",
                    "value": invoice.amount.to_string()
                },
                {
                    "trait_type": "Currency",
                    "value": invoice.currency
                },
                {
                    "trait_type": "Buyer Country",
                    "value": invoice.buyer_country
                },
                {
                    "trait_type": "Grade",
                    "value": invoice.grade.unwrap_or_default()
                },
                {
                    "trait_type": "Due Date",
                    "value": invoice.due_date.to_string()
                }
            ]
        });

        let metadata_uri = self
            .pinata_service
            .upload_json(metadata, &format!("vessel-invoice-{}", invoice_id))
            .await?;

        Ok(metadata_uri)
    }

    pub async fn record_investment_on_chain(
        &self,
        token_id: i64,
        investor_address: &str,
        amount: Decimal,
    ) -> AppResult<String> {
        if self.config.skip_blockchain_verification {
            tracing::info!("SKIPPING blockchain investment recording (Test Mode)");
            return Ok("0xTestRecordInvestHash".to_string());
        }

        let wallet = self.wallet.as_ref().ok_or_else(|| {
            AppError::BlockchainError("Platform wallet not configured".to_string())
        })?;

        let contract_addr: Address =
            self.config
                .invoice_pool_contract_addr
                .parse()
                .map_err(|_| {
                    AppError::BlockchainError("Invalid InvoicePool contract address".to_string())
                })?;

        let client = SignerMiddleware::new(self.provider.clone(), wallet.clone());
        let contract = InvoicePool::new(contract_addr, Arc::new(client));

        let investor_addr: Address = investor_address
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid investor address".to_string()))?;

        let amount_units = self.to_token_units(amount);

        tracing::info!(
            "Recording investment on-chain: token {} from {} amount {}",
            token_id,
            investor_address,
            amount
        );

        let tx = contract.record_investment(U256::from(token_id), investor_addr, amount_units);

        let pending_tx = tx.send().await.map_err(|e| {
            AppError::BlockchainError(format!("Failed to send record investment tx: {}", e))
        })?;

        let receipt = pending_tx
            .await
            .map_err(|e| {
                AppError::BlockchainError(format!(
                    "Failed to wait for record investment receipt: {}",
                    e
                ))
            })?
            .ok_or_else(|| {
                AppError::BlockchainError("Record investment transaction failed".to_string())
            })?;

        Ok(format!("{:?}", receipt.transaction_hash))
    }

    pub async fn record_repayment_on_chain(
        &self,
        token_id: i64,
        total_amount: Decimal,
        investor_returns: Vec<Decimal>,
    ) -> AppResult<String> {
        if self.config.skip_blockchain_verification {
            tracing::info!("SKIPPING blockchain repayment recording (Test Mode)");
            return Ok("0xTestRecordRepayHash".to_string());
        }

        let wallet = self.wallet.as_ref().ok_or_else(|| {
            AppError::BlockchainError("Platform wallet not configured".to_string())
        })?;

        let contract_addr: Address =
            self.config
                .invoice_pool_contract_addr
                .parse()
                .map_err(|_| {
                    AppError::BlockchainError("Invalid InvoicePool contract address".to_string())
                })?;

        let client = SignerMiddleware::new(self.provider.clone(), wallet.clone());
        let contract = InvoicePool::new(contract_addr, Arc::new(client));

        let total_amount_units = self.to_token_units(total_amount);
        let returns_units: Vec<U256> = investor_returns
            .iter()
            .map(|&amount| self.to_token_units(amount))
            .collect();

        tracing::info!(
            "Recording repayment on-chain: token {} amount {}",
            token_id,
            total_amount
        );

        let tx = contract.record_repayment(U256::from(token_id), total_amount_units, returns_units);

        let pending_tx = tx.send().await.map_err(|e| {
            AppError::BlockchainError(format!("Failed to send record repayment tx: {}", e))
        })?;

        let receipt = pending_tx
            .await
            .map_err(|e| {
                AppError::BlockchainError(format!(
                    "Failed to wait for record repayment receipt: {}",
                    e
                ))
            })?
            .ok_or_else(|| {
                AppError::BlockchainError("Record repayment transaction failed".to_string())
            })?;

        Ok(format!("{:?}", receipt.transaction_hash))
    }

    pub async fn create_pool_on_chain(&self, token_id: i64) -> AppResult<String> {
        if self.config.skip_blockchain_verification {
            tracing::info!("SKIPPING blockchain pool creation (Test Mode)");
            return Ok("0xTestCreatePoolHash".to_string());
        }

        let wallet = self.wallet.as_ref().ok_or_else(|| {
            AppError::BlockchainError("Platform wallet not configured".to_string())
        })?;

        let contract_addr: Address =
            self.config
                .invoice_pool_contract_addr
                .parse()
                .map_err(|_| {
                    AppError::BlockchainError("Invalid InvoicePool contract address".to_string())
                })?;

        let client = SignerMiddleware::new(self.provider.clone(), wallet.clone());
        let contract = InvoicePool::new(contract_addr, Arc::new(client));

        tracing::info!("Creating pool on-chain for token {}", token_id);

        let tx = contract.create_pool(U256::from(token_id));

        let pending_tx = tx.send().await.map_err(|e| {
            AppError::BlockchainError(format!("Failed to send createPool tx: {}", e))
        })?;

        let receipt = pending_tx
            .await
            .map_err(|e| {
                AppError::BlockchainError(format!("Failed to wait for createPool receipt: {}", e))
            })?
            .ok_or_else(|| {
                AppError::BlockchainError("createPool transaction failed".to_string())
            })?;

        Ok(format!("{:?}", receipt.transaction_hash))
    }

    pub async fn close_pool_on_chain(&self, token_id: i64) -> AppResult<String> {
        if self.config.skip_blockchain_verification {
            tracing::info!("SKIPPING blockchain pool close (Test Mode)");
            return Ok("0xTestClosePoolHash".to_string());
        }

        let wallet = self.wallet.as_ref().ok_or_else(|| {
            AppError::BlockchainError("Platform wallet not configured".to_string())
        })?;

        let contract_addr: Address =
            self.config
                .invoice_pool_contract_addr
                .parse()
                .map_err(|_| {
                    AppError::BlockchainError("Invalid InvoicePool contract address".to_string())
                })?;

        let client = SignerMiddleware::new(self.provider.clone(), wallet.clone());
        let contract = InvoicePool::new(contract_addr, Arc::new(client));

        tracing::info!("Closing pool on-chain for token {}", token_id);

        let tx = contract.close_pool_early(U256::from(token_id));

        let pending_tx = tx.send().await.map_err(|e| {
            AppError::BlockchainError(format!("Failed to send closePoolEarly tx: {}", e))
        })?;

        let receipt = pending_tx
            .await
            .map_err(|e| {
                AppError::BlockchainError(format!(
                    "Failed to wait for closePoolEarly receipt: {}",
                    e
                ))
            })?
            .ok_or_else(|| {
                AppError::BlockchainError("closePoolEarly transaction failed".to_string())
            })?;

        Ok(format!("{:?}", receipt.transaction_hash))
    }
}
