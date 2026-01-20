use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{MitraApplication, MitraApplyRequest, MitraStatusResponse, MitraDocumentsStatus};
use crate::repository::{MitraRepository, UserRepository};

use super::{EmailService, PinataService};

pub struct MitraService {
    mitra_repo: Arc<MitraRepository>,
    user_repo: Arc<UserRepository>,
    email_service: Arc<EmailService>,
    pinata_service: Arc<PinataService>,
}

impl MitraService {
    pub fn new(
        mitra_repo: Arc<MitraRepository>,
        user_repo: Arc<UserRepository>,
        email_service: Arc<EmailService>,
        pinata_service: Arc<PinataService>,
    ) -> Self {
        Self {
            mitra_repo,
            user_repo,
            email_service,
            pinata_service,
        }
    }

    pub async fn apply(&self, user_id: Uuid, req: MitraApplyRequest) -> AppResult<MitraApplication> {
        // Check if user already has an application
        if let Some(existing) = self.mitra_repo.find_by_user(user_id).await? {
            if existing.status == "pending" {
                return Err(AppError::Conflict("Application already pending".to_string()));
            }
            if existing.status == "approved" {
                return Err(AppError::Conflict("Already approved as mitra".to_string()));
            }
        }

        // Create application
        let application = self.mitra_repo.create(
            user_id,
            &req.company_name,
            req.company_type.as_deref().unwrap_or("PT"),
            &req.npwp,
            &req.annual_revenue,
            req.address.as_deref(),
            req.business_description.as_deref(),
            req.website_url.as_deref(),
            req.year_founded,
            req.key_products.as_deref(),
            req.export_markets.as_deref(),
        ).await?;

        Ok(application)
    }

    pub async fn get_status(&self, user_id: Uuid) -> AppResult<MitraStatusResponse> {
        let application = self.mitra_repo.find_by_user(user_id).await?;

        match application {
            Some(app) => {
                let docs_status = MitraDocumentsStatus {
                    nib_uploaded: app.nib_document_url.is_some(),
                    akta_pendirian_uploaded: app.akta_pendirian_url.is_some(),
                    ktp_direktur_uploaded: app.ktp_direktur_url.is_some(),
                    all_documents_complete: app.nib_document_url.is_some()
                        && app.akta_pendirian_url.is_some()
                        && app.ktp_direktur_url.is_some(),
                };

                Ok(MitraStatusResponse {
                    status: app.status.clone(),
                    application: Some(app.clone()),
                    rejection_reason: app.rejection_reason.clone(),
                    reviewed_at: app.reviewed_at,
                    documents_status: docs_status,
                })
            }
            None => Ok(MitraStatusResponse {
                status: "none".to_string(),
                application: None,
                rejection_reason: None,
                reviewed_at: None,
                documents_status: MitraDocumentsStatus {
                    nib_uploaded: false,
                    akta_pendirian_uploaded: false,
                    ktp_direktur_uploaded: false,
                    all_documents_complete: false,
                },
            }),
        }
    }

    pub async fn upload_document(
        &self,
        user_id: Uuid,
        document_type: &str,
        file_data: Vec<u8>,
        file_name: &str,
    ) -> AppResult<MitraApplication> {
        let application = self.mitra_repo.find_by_user(user_id).await?
            .ok_or_else(|| AppError::NotFound("No mitra application found".to_string()))?;

        if application.status != "pending" {
            return Err(AppError::BadRequest("Cannot modify approved/rejected application".to_string()));
        }

        // Upload to IPFS
        let file_url = self.pinata_service.upload_file(file_data, file_name).await?;

        // Update application
        self.mitra_repo.update_document(application.id, document_type, &file_url).await
    }

    pub async fn get_pending_applications(&self, page: i32, per_page: i32) -> AppResult<(Vec<MitraApplication>, i64)> {
        self.mitra_repo.find_pending(page, per_page).await
    }

    pub async fn get_application(&self, id: Uuid) -> AppResult<MitraApplication> {
        self.mitra_repo.find_by_id(id).await?
            .ok_or_else(|| AppError::NotFound("Application not found".to_string()))
    }

    pub async fn approve(&self, id: Uuid, admin_id: Uuid) -> AppResult<MitraApplication> {
        let application = self.get_application(id).await?;

        if application.status != "pending" {
            return Err(AppError::BadRequest("Application is not pending".to_string()));
        }

        // Approve application
        let approved = self.mitra_repo.approve(id, admin_id).await?;

        // Update user role to mitra
        self.user_repo.update_role(application.user_id, "mitra").await?;
        self.user_repo.update_member_status(application.user_id, "member_mitra").await?;

        // Send notification email
        if let Some(user) = self.user_repo.find_by_id(application.user_id).await? {
            let _ = self.email_service.send_mitra_approval_notification(
                &user.email,
                &approved.company_name,
            ).await;
        }

        Ok(approved)
    }

    pub async fn reject(&self, id: Uuid, admin_id: Uuid, reason: &str) -> AppResult<MitraApplication> {
        let application = self.get_application(id).await?;

        if application.status != "pending" {
            return Err(AppError::BadRequest("Application is not pending".to_string()));
        }

        self.mitra_repo.reject(id, admin_id, reason).await
    }
}
