use chrono::NaiveDate;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::{
    AdminGradeSuggestionResponse, CreateInvoiceFundingRequest, Invoice, InvoiceDocument,
    RepeatBuyerCheckResponse,
};
use crate::repository::{FundingRepository, InvoiceRepository, MitraRepository, UserRepository};

use super::PinataService;

pub struct InvoiceService {
    invoice_repo: Arc<InvoiceRepository>,
    funding_repo: Arc<FundingRepository>,
    user_repo: Arc<UserRepository>,
    mitra_repo: Arc<MitraRepository>,
    pinata_service: Arc<PinataService>,
    config: Arc<Config>,
}

impl InvoiceService {
    pub fn new(
        invoice_repo: Arc<InvoiceRepository>,
        funding_repo: Arc<FundingRepository>,
        user_repo: Arc<UserRepository>,
        mitra_repo: Arc<MitraRepository>,
        pinata_service: Arc<PinataService>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            invoice_repo,
            funding_repo,
            user_repo,
            mitra_repo,
            pinata_service,
            config,
        }
    }

    pub async fn create_funding_request(
        &self,
        exporter_id: Uuid,
        req: CreateInvoiceFundingRequest,
    ) -> AppResult<Invoice> {
        // Check if mitra is approved
        let mitra = self
            .mitra_repo
            .find_by_user(exporter_id)
            .await?
            .ok_or_else(|| AppError::Forbidden("Must be an approved mitra".to_string()))?;

        if mitra.status != "approved" {
            return Err(AppError::Forbidden(
                "Mitra application not yet approved".to_string(),
            ));
        }

        // Validate data confirmation
        if !req.data_confirmation {
            return Err(AppError::ValidationError(
                "Must confirm data accuracy".to_string(),
            ));
        }

        // Parse due date
        let due_date = NaiveDate::parse_from_str(&req.due_date, "%Y-%m-%d")
            .map_err(|_| AppError::ValidationError("Invalid due date format".to_string()))?;

        // Create invoice
        let amount = Decimal::from_f64(req.idr_amount)
            .ok_or_else(|| AppError::ValidationError("Invalid amount".to_string()))?;

        let invoice = self
            .invoice_repo
            .create(
                exporter_id,
                &req.buyer_company_name,
                &req.buyer_country,
                Some(&req.buyer_email),
                &req.invoice_number,
                "IDR",
                amount,
                chrono::Utc::now().date_naive(),
                due_date,
                req.description.as_deref(),
                &req.wallet_address,
            )
            .await?;

        // Update with additional fields
        self.invoice_repo
            .set_repeat_buyer(invoice.id, req.is_repeat_buyer)
            .await?;

        // Update interest rates
        let priority_rate = Decimal::from_f64(req.priority_interest_rate)
            .ok_or_else(|| AppError::ValidationError("Invalid priority rate".to_string()))?;
        let catalyst_rate = Decimal::from_f64(req.catalyst_interest_rate)
            .ok_or_else(|| AppError::ValidationError("Invalid catalyst rate".to_string()))?;

        self.invoice_repo
            .update_interest_rates(invoice.id, priority_rate, catalyst_rate)
            .await?;

        // Submit for review
        self.invoice_repo
            .update_status(invoice.id, "pending_review")
            .await?;

        self.invoice_repo
            .find_by_id(invoice.id)
            .await?
            .ok_or_else(|| AppError::InternalError("Failed to fetch created invoice".to_string()))
    }

    pub async fn get_invoice(&self, id: Uuid) -> AppResult<Invoice> {
        let invoice = self
            .invoice_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        Ok(invoice)
    }

    pub async fn list_by_exporter(
        &self,
        exporter_id: Uuid,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<Invoice>, i64)> {
        self.invoice_repo
            .find_by_exporter(exporter_id, page, per_page)
            .await
    }

    pub async fn list_fundable(&self, page: i32, per_page: i32) -> AppResult<(Vec<Invoice>, i64)> {
        self.invoice_repo.find_fundable(page, per_page).await
    }

    pub async fn list_pending(&self, page: i32, per_page: i32) -> AppResult<(Vec<Invoice>, i64)> {
        self.invoice_repo
            .find_by_status("pending_review", page, per_page)
            .await
    }

    pub async fn list_approved(&self, page: i32, per_page: i32) -> AppResult<(Vec<Invoice>, i64)> {
        self.invoice_repo
            .find_by_status("approved", page, per_page)
            .await
    }

    pub async fn approve(
        &self,
        id: Uuid,
        grade: &str,
        priority_rate: Option<f64>,
        catalyst_rate: Option<f64>,
    ) -> AppResult<Invoice> {
        let invoice = self.get_invoice(id).await?;

        if invoice.status != "pending_review" {
            return Err(AppError::BadRequest(
                "Invoice is not pending review".to_string(),
            ));
        }

        // Calculate grade score
        let grade_score = match grade {
            "A" => 90,
            "B" => 70,
            "C" => 50,
            _ => return Err(AppError::ValidationError("Invalid grade".to_string())),
        };

        // Determine funding limit based on repeat buyer status
        let funding_limit = if invoice.is_repeat_buyer {
            Decimal::from(100)
        } else {
            Decimal::from(60)
        };

        // Update grade
        self.invoice_repo
            .update_grade(id, grade, grade_score, funding_limit)
            .await?;

        // Update interest rates if provided
        if let (Some(pr), Some(cr)) = (priority_rate, catalyst_rate) {
            let priority = Decimal::from_f64(pr)
                .ok_or_else(|| AppError::ValidationError("Invalid priority rate".to_string()))?;
            let catalyst = Decimal::from_f64(cr)
                .ok_or_else(|| AppError::ValidationError("Invalid catalyst rate".to_string()))?;
            self.invoice_repo
                .update_interest_rates(id, priority, catalyst)
                .await?;
        }

        // Update status to approved
        self.invoice_repo.update_status(id, "approved").await
    }

    pub async fn reject(&self, id: Uuid, _reason: &str) -> AppResult<Invoice> {
        let invoice = self.get_invoice(id).await?;

        if invoice.status != "pending_review" {
            return Err(AppError::BadRequest(
                "Invoice is not pending review".to_string(),
            ));
        }

        self.invoice_repo.update_status(id, "rejected").await
    }

    pub async fn get_grade_suggestion(&self, id: Uuid) -> AppResult<AdminGradeSuggestionResponse> {
        let invoice = self.get_invoice(id).await?;

        // Calculate country risk score
        let country_score = self.calculate_country_score(&invoice.buyer_country);
        let country_risk = if country_score >= 30 {
            "low"
        } else if country_score >= 20 {
            "medium"
        } else {
            "high"
        };

        // Calculate history score
        let history_score = if invoice.is_repeat_buyer { 30 } else { 0 };

        // Calculate document score
        let documents = self.invoice_repo.find_documents_by_invoice(id).await?;
        let document_score = self.calculate_document_score(&documents);

        // Total score
        let total_score = country_score + history_score + document_score;

        // Determine grade
        let suggested_grade = if total_score >= 75 {
            "A"
        } else if total_score >= 50 {
            "B"
        } else {
            "C"
        };

        let funding_limit = if invoice.is_repeat_buyer { 100.0 } else { 60.0 };

        Ok(AdminGradeSuggestionResponse {
            invoice_id: id.to_string(),
            suggested_grade: suggested_grade.to_string(),
            grade_score: total_score,
            country_risk: country_risk.to_string(),
            country_score,
            history_score,
            document_score,
            is_repeat_buyer: invoice.is_repeat_buyer,
            documents_complete: document_score >= 30,
            funding_limit,
        })
    }

    pub async fn check_repeat_buyer(
        &self,
        exporter_id: Uuid,
        buyer_name: &str,
    ) -> AppResult<RepeatBuyerCheckResponse> {
        let count = self
            .invoice_repo
            .count_by_buyer_name(exporter_id, buyer_name)
            .await?;

        let is_repeat = count > 0;
        let funding_limit = if is_repeat { 100.0 } else { 60.0 };

        Ok(RepeatBuyerCheckResponse {
            is_repeat_buyer: is_repeat,
            message: if is_repeat {
                format!("Buyer has {} previous successful transactions", count)
            } else {
                "New buyer - funding limited to 60%".to_string()
            },
            previous_transactions: if is_repeat { Some(count as i32) } else { None },
            funding_limit,
        })
    }

    pub async fn upload_document(
        &self,
        invoice_id: Uuid,
        document_type: &str,
        file_name: &str,
        file_data: Vec<u8>,
    ) -> AppResult<InvoiceDocument> {
        // Upload to IPFS
        let file_url = self
            .pinata_service
            .upload_file(file_data.clone(), file_name)
            .await?;

        // Calculate hash
        let file_hash = format!("{:x}", md5::compute(&file_data));

        // Create document record
        self.invoice_repo
            .create_document(
                invoice_id,
                document_type,
                file_name,
                &file_url,
                &file_hash,
                file_data.len() as i32,
            )
            .await
    }

    pub async fn get_documents(&self, invoice_id: Uuid) -> AppResult<Vec<InvoiceDocument>> {
        self.invoice_repo
            .find_documents_by_invoice(invoice_id)
            .await
    }

    fn calculate_country_score(&self, country: &str) -> i32 {
        // Tier 1 countries (low risk)
        let tier1 = vec![
            "USA",
            "Germany",
            "Japan",
            "United Kingdom",
            "France",
            "Switzerland",
            "Netherlands",
            "Australia",
            "Canada",
            "Singapore",
            "South Korea",
        ];
        // Tier 2 countries (medium risk)
        let tier2 = vec![
            "China",
            "India",
            "Brazil",
            "Mexico",
            "Thailand",
            "Malaysia",
            "Vietnam",
            "Philippines",
            "Indonesia",
            "Turkey",
            "Saudi Arabia",
            "UAE",
        ];

        if tier1
            .iter()
            .any(|c| country.to_lowercase().contains(&c.to_lowercase()))
        {
            35 // High score for tier 1
        } else if tier2
            .iter()
            .any(|c| country.to_lowercase().contains(&c.to_lowercase()))
        {
            25 // Medium score for tier 2
        } else {
            15 // Lower score for tier 3
        }
    }

    fn calculate_document_score(&self, documents: &[InvoiceDocument]) -> i32 {
        let mut score = 0;

        let required_types = vec!["invoice_pdf", "bill_of_lading", "purchase_order"];
        let optional_types = vec!["packing_list", "certificate_of_origin", "insurance"];

        for doc_type in required_types {
            if documents.iter().any(|d| d.document_type == doc_type) {
                score += 10;
            }
        }

        for doc_type in optional_types {
            if documents.iter().any(|d| d.document_type == doc_type) {
                score += 5;
            }
        }

        score.min(35) // Cap at 35
    }
}
