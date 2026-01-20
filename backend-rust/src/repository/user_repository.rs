use sqlx::PgPool;
use uuid::Uuid;
use rust_decimal::Decimal;

use crate::error::AppResult;
use crate::models::{User, UserProfile, BankAccount, UserIdentity};

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, email: &str, username: &str, password_hash: &str, role: &str) -> AppResult<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (email, username, password_hash, role, is_verified, is_active, cooperative_agreement, member_status, balance_idr, email_verified, profile_completed)
            VALUES ($1, $2, $3, $4, false, true, false, 'calon_anggota_pendana', 0, true, false)
            RETURNING *
            "#,
        )
        .bind(email)
        .bind(username)
        .bind(password_hash)
        .bind(role)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    pub async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    pub async fn find_by_username(&self, username: &str) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    pub async fn find_by_email_or_username(&self, email_or_username: &str) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE email = $1 OR username = $1"
        )
        .bind(email_or_username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn find_by_wallet_address(&self, wallet_address: &str) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE wallet_address = $1")
            .bind(wallet_address.to_lowercase())
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    /// Alias for find_by_wallet_address
    pub async fn find_by_wallet(&self, wallet_address: &str) -> AppResult<Option<User>> {
        self.find_by_wallet_address(wallet_address).await
    }

    /// Create investor account with wallet only (no email/password required)
    pub async fn create_investor_with_wallet(&self, wallet_address: &str) -> AppResult<User> {
        let wallet = wallet_address.to_lowercase();
        // Generate a unique placeholder email based on wallet address
        let email = format!("{}@wallet.vessel.io", &wallet[2..10]);

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (
                email, username, password_hash, role, is_verified, is_active,
                cooperative_agreement, member_status, balance_idr, email_verified,
                profile_completed, wallet_address
            )
            VALUES ($1, $2, '', 'investor', true, true, true, 'calon_anggota_pendana', 0, false, false, $3)
            RETURNING *
            "#,
        )
        .bind(&email)
        .bind(&wallet[2..10]) // Use part of wallet as username
        .bind(&wallet)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn update_password(&self, user_id: Uuid, password_hash: &str) -> AppResult<()> {
        sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
            .bind(password_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_wallet_address(&self, user_id: Uuid, wallet_address: &str) -> AppResult<()> {
        sqlx::query("UPDATE users SET wallet_address = $1, updated_at = NOW() WHERE id = $2")
            .bind(wallet_address)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_balance(&self, user_id: Uuid, new_balance: Decimal) -> AppResult<()> {
        sqlx::query("UPDATE users SET balance_idr = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_balance)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn set_profile_completed(&self, user_id: Uuid, completed: bool) -> AppResult<()> {
        sqlx::query("UPDATE users SET profile_completed = $1, updated_at = NOW() WHERE id = $2")
            .bind(completed)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn set_verified(&self, user_id: Uuid, verified: bool) -> AppResult<()> {
        sqlx::query("UPDATE users SET is_verified = $1, updated_at = NOW() WHERE id = $2")
            .bind(verified)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_role(&self, user_id: Uuid, role: &str) -> AppResult<()> {
        sqlx::query("UPDATE users SET role = $1, updated_at = NOW() WHERE id = $2")
            .bind(role)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_member_status(&self, user_id: Uuid, status: &str) -> AppResult<()> {
        sqlx::query("UPDATE users SET member_status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn list_all(&self, page: i32, per_page: i32) -> AppResult<(Vec<User>, i64)> {
        let offset = (page - 1) * per_page;

        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        Ok((users, total.0))
    }

    // Profile methods
    pub async fn create_profile(&self, user_id: Uuid, full_name: &str) -> AppResult<UserProfile> {
        let profile = sqlx::query_as::<_, UserProfile>(
            r#"
            INSERT INTO user_profiles (user_id, full_name)
            VALUES ($1, $2)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(full_name)
        .fetch_one(&self.pool)
        .await?;

        Ok(profile)
    }

    pub async fn find_profile_by_user_id(&self, user_id: Uuid) -> AppResult<Option<UserProfile>> {
        let profile = sqlx::query_as::<_, UserProfile>(
            "SELECT * FROM user_profiles WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(profile)
    }

    pub async fn update_profile(
        &self,
        user_id: Uuid,
        full_name: Option<&str>,
        phone: Option<&str>,
        country: Option<&str>,
        company_name: Option<&str>,
        company_type: Option<&str>,
        business_sector: Option<&str>,
    ) -> AppResult<UserProfile> {
        let profile = sqlx::query_as::<_, UserProfile>(
            r#"
            UPDATE user_profiles
            SET full_name = COALESCE($2, full_name),
                phone = COALESCE($3, phone),
                country = COALESCE($4, country),
                company_name = COALESCE($5, company_name),
                company_type = COALESCE($6, company_type),
                business_sector = COALESCE($7, business_sector),
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(full_name)
        .bind(phone)
        .bind(country)
        .bind(company_name)
        .bind(company_type)
        .bind(business_sector)
        .fetch_one(&self.pool)
        .await?;

        Ok(profile)
    }

    // Bank account methods
    pub async fn create_bank_account(
        &self,
        user_id: Uuid,
        bank_code: &str,
        bank_name: &str,
        account_number: &str,
        account_name: &str,
    ) -> AppResult<BankAccount> {
        let account = sqlx::query_as::<_, BankAccount>(
            r#"
            INSERT INTO bank_accounts (user_id, bank_code, bank_name, account_number, account_name, is_primary)
            VALUES ($1, $2, $3, $4, $5, true)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(bank_code)
        .bind(bank_name)
        .bind(account_number)
        .bind(account_name)
        .fetch_one(&self.pool)
        .await?;

        Ok(account)
    }

    pub async fn find_primary_bank_account(&self, user_id: Uuid) -> AppResult<Option<BankAccount>> {
        let account = sqlx::query_as::<_, BankAccount>(
            "SELECT * FROM bank_accounts WHERE user_id = $1 AND is_primary = true"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(account)
    }

    pub async fn update_bank_account(
        &self,
        user_id: Uuid,
        bank_code: &str,
        bank_name: &str,
        account_number: &str,
        account_name: &str,
    ) -> AppResult<BankAccount> {
        let account = sqlx::query_as::<_, BankAccount>(
            r#"
            UPDATE bank_accounts
            SET bank_code = $2, bank_name = $3, account_number = $4, account_name = $5, updated_at = NOW()
            WHERE user_id = $1 AND is_primary = true
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(bank_code)
        .bind(bank_name)
        .bind(account_number)
        .bind(account_name)
        .fetch_one(&self.pool)
        .await?;

        Ok(account)
    }

    // Identity methods
    pub async fn create_identity(
        &self,
        user_id: Uuid,
        nik: &str,
        full_name: &str,
        ktp_photo_url: &str,
        selfie_url: &str,
    ) -> AppResult<UserIdentity> {
        let identity = sqlx::query_as::<_, UserIdentity>(
            r#"
            INSERT INTO user_identities (user_id, nik, full_name, ktp_photo_url, selfie_url)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(nik)
        .bind(full_name)
        .bind(ktp_photo_url)
        .bind(selfie_url)
        .fetch_one(&self.pool)
        .await?;

        Ok(identity)
    }

    pub async fn find_identity_by_user_id(&self, user_id: Uuid) -> AppResult<Option<UserIdentity>> {
        let identity = sqlx::query_as::<_, UserIdentity>(
            "SELECT * FROM user_identities WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(identity)
    }

    // Additional methods needed by handlers
    pub async fn update_wallet(&self, user_id: Uuid, wallet_address: &str) -> AppResult<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users SET wallet_address = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(wallet_address)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn list_users(
        &self,
        role: Option<&str>,
        page: i32,
        per_page: i32,
    ) -> AppResult<(Vec<User>, i64)> {
        let offset = (page - 1) * per_page;

        let (users, total) = if let Some(role) = role {
            let users = sqlx::query_as::<_, User>(
                "SELECT * FROM users WHERE role = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            )
            .bind(role)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

            let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE role = $1")
                .bind(role)
                .fetch_one(&self.pool)
                .await?;

            (users, total.0)
        } else {
            let users = sqlx::query_as::<_, User>(
                "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2"
            )
            .bind(per_page)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

            let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
                .fetch_one(&self.pool)
                .await?;

            (users, total.0)
        };

        Ok((users, total))
    }
}
