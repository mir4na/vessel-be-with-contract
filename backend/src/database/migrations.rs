use sqlx::PgPool;
use tracing::{info, warn};
use anyhow::Result;

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    let migrations = vec![
        // Enable UUID extension
        r#"CREATE EXTENSION IF NOT EXISTS "uuid-ossp";"#,

        // Users table
        r#"CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            role VARCHAR(20) NOT NULL CHECK (role IN ('exporter', 'investor', 'admin', 'mitra')),
            wallet_address VARCHAR(42) UNIQUE,
            is_verified BOOLEAN DEFAULT false,
            is_active BOOLEAN DEFAULT true,
            username VARCHAR(50) UNIQUE,
            phone_number VARCHAR(20),
            cooperative_agreement BOOLEAN DEFAULT false,
            member_status VARCHAR(30) DEFAULT 'calon_anggota_pendana',
            balance_idr DECIMAL(20,2) DEFAULT 0,
            email_verified BOOLEAN DEFAULT false,
            profile_completed BOOLEAN DEFAULT false,
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // User profiles table
        r#"CREATE TABLE IF NOT EXISTS user_profiles (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            user_id UUID UNIQUE REFERENCES users(id) ON DELETE CASCADE,
            full_name VARCHAR(255) NOT NULL,
            phone VARCHAR(20),
            country VARCHAR(100),
            company_name VARCHAR(255),
            company_type VARCHAR(100),
            business_sector VARCHAR(100),
            avatar_url TEXT,
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // KYC verifications table
        r#"CREATE TABLE IF NOT EXISTS kyc_verifications (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            verification_type VARCHAR(10) NOT NULL CHECK (verification_type IN ('kyc', 'kyb')),
            status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
            id_type VARCHAR(50),
            id_number VARCHAR(100),
            id_document_url TEXT,
            selfie_url TEXT,
            rejection_reason TEXT,
            verified_by UUID REFERENCES users(id),
            verified_at TIMESTAMP,
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Invoices table
        r#"CREATE TABLE IF NOT EXISTS invoices (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            exporter_id UUID REFERENCES users(id) ON DELETE CASCADE NOT NULL,
            buyer_name VARCHAR(255) NOT NULL,
            buyer_country VARCHAR(100) NOT NULL,
            buyer_email VARCHAR(255),
            invoice_number VARCHAR(100) NOT NULL,
            currency VARCHAR(10) DEFAULT 'USD',
            amount DECIMAL(20,2) NOT NULL,
            issue_date DATE NOT NULL,
            due_date DATE NOT NULL,
            description TEXT,
            status VARCHAR(30) NOT NULL DEFAULT 'draft' CHECK (status IN (
                'draft', 'pending_review', 'approved', 'rejected',
                'tokenized', 'funding', 'funded', 'matured', 'repaid', 'defaulted'
            )),
            interest_rate DECIMAL(5,2),
            advance_percentage DECIMAL(5,2) DEFAULT 80.00,
            advance_amount DECIMAL(20,2),
            document_hash VARCHAR(66),
            -- Grading fields
            grade VARCHAR(1),
            grade_score INTEGER,
            is_repeat_buyer BOOLEAN DEFAULT false,
            funding_limit_percentage DECIMAL(5,2) DEFAULT 100.00,
            is_insured BOOLEAN DEFAULT false,
            document_complete_score INTEGER DEFAULT 0,
            buyer_country_risk VARCHAR(10),
            -- Tranche fields
            priority_ratio DECIMAL(5,2) DEFAULT 80.00,
            catalyst_ratio DECIMAL(5,2) DEFAULT 20.00,
            priority_interest_rate DECIMAL(5,2),
            catalyst_interest_rate DECIMAL(5,2),
            -- Currency fields
            original_currency VARCHAR(10) DEFAULT 'USD',
            original_amount DECIMAL(20,2),
            idrx_amount DECIMAL(20,2),
            exchange_rate DECIMAL(15,6),
            buffer_rate DECIMAL(5,4) DEFAULT 0.015,
            -- Additional fields
            funding_duration_days INTEGER DEFAULT 14,
            payment_link TEXT,
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Invoice documents table
        r#"CREATE TABLE IF NOT EXISTS invoice_documents (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            invoice_id UUID REFERENCES invoices(id) ON DELETE CASCADE,
            document_type VARCHAR(30) NOT NULL CHECK (document_type IN (
                'invoice_pdf', 'bill_of_lading', 'packing_list',
                'certificate_of_origin', 'insurance', 'customs', 'other',
                'purchase_order', 'commercial_invoice'
            )),
            file_name VARCHAR(255),
            file_url TEXT,
            file_hash VARCHAR(66),
            file_size INTEGER,
            uploaded_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Invoice NFTs table
        r#"CREATE TABLE IF NOT EXISTS invoice_nfts (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            invoice_id UUID UNIQUE REFERENCES invoices(id) ON DELETE CASCADE,
            token_id BIGINT,
            contract_address VARCHAR(42),
            chain_id INTEGER,
            owner_address VARCHAR(42),
            mint_tx_hash VARCHAR(66),
            metadata_uri TEXT,
            minted_at TIMESTAMP,
            burned_at TIMESTAMP,
            burn_tx_hash VARCHAR(66),
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Funding pools table
        r#"CREATE TABLE IF NOT EXISTS funding_pools (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            invoice_id UUID UNIQUE REFERENCES invoices(id) ON DELETE CASCADE,
            target_amount DECIMAL(20,2) NOT NULL,
            funded_amount DECIMAL(20,2) DEFAULT 0,
            investor_count INTEGER DEFAULT 0,
            status VARCHAR(20) NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'filled', 'disbursed', 'closed')),
            opened_at TIMESTAMP,
            deadline TIMESTAMP,
            filled_at TIMESTAMP,
            disbursed_at TIMESTAMP,
            closed_at TIMESTAMP,
            -- Tranche fields
            priority_target DECIMAL(20,2),
            priority_funded DECIMAL(20,2) DEFAULT 0,
            catalyst_target DECIMAL(20,2),
            catalyst_funded DECIMAL(20,2) DEFAULT 0,
            priority_interest_rate DECIMAL(5,2),
            catalyst_interest_rate DECIMAL(5,2),
            pool_currency VARCHAR(10) DEFAULT 'IDRX',
            create_pool_tx_hash VARCHAR(66),
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Investments table
        r#"CREATE TABLE IF NOT EXISTS investments (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            pool_id UUID REFERENCES funding_pools(id) ON DELETE CASCADE,
            investor_id UUID REFERENCES users(id) ON DELETE CASCADE,
            amount DECIMAL(20,2) NOT NULL,
            expected_return DECIMAL(20,2),
            actual_return DECIMAL(20,2),
            status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'repaid', 'defaulted')),
            tranche VARCHAR(10) DEFAULT 'priority',
            tx_hash VARCHAR(66),
            return_tx_hash VARCHAR(66),
            invested_at TIMESTAMP DEFAULT NOW(),
            repaid_at TIMESTAMP,
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Transactions table
        r#"CREATE TABLE IF NOT EXISTS transactions (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            invoice_id UUID REFERENCES invoices(id),
            user_id UUID REFERENCES users(id),
            type VARCHAR(30) NOT NULL CHECK (type IN (
                'investment', 'advance_payment', 'buyer_repayment',
                'investor_return', 'platform_fee', 'refund'
            )),
            amount DECIMAL(20,2) NOT NULL,
            currency VARCHAR(10) DEFAULT 'IDR',
            tx_hash VARCHAR(66),
            status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'confirmed', 'failed')),
            from_address VARCHAR(42),
            to_address VARCHAR(42),
            block_number BIGINT,
            gas_used BIGINT,
            notes TEXT,
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // OTP codes table
        r#"CREATE TABLE IF NOT EXISTS otp_codes (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            email VARCHAR(255) NOT NULL,
            code VARCHAR(6) NOT NULL,
            purpose VARCHAR(20) NOT NULL CHECK (purpose IN ('registration', 'login', 'password_reset')),
            expires_at TIMESTAMP NOT NULL,
            verified BOOLEAN DEFAULT false,
            attempts INTEGER DEFAULT 0,
            created_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Mitra applications table
        r#"CREATE TABLE IF NOT EXISTS mitra_applications (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            user_id UUID UNIQUE REFERENCES users(id) ON DELETE CASCADE,
            company_name VARCHAR(255) NOT NULL,
            company_type VARCHAR(50) NOT NULL DEFAULT 'PT',
            npwp VARCHAR(16) NOT NULL,
            annual_revenue VARCHAR(50) NOT NULL,
            nib_document_url TEXT,
            akta_pendirian_url TEXT,
            ktp_direktur_url TEXT,
            status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
            rejection_reason TEXT,
            reviewed_by UUID REFERENCES users(id),
            reviewed_at TIMESTAMP,
            address TEXT,
            business_description TEXT,
            website_url VARCHAR(255),
            year_founded INTEGER,
            key_products TEXT,
            export_markets TEXT,
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Risk questionnaires table
        r#"CREATE TABLE IF NOT EXISTS risk_questionnaires (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            user_id UUID UNIQUE REFERENCES users(id) ON DELETE CASCADE,
            q1_answer INTEGER CHECK (q1_answer IN (1, 2, 3)),
            q2_answer INTEGER CHECK (q2_answer IN (1, 2)),
            q3_answer INTEGER CHECK (q3_answer IN (1, 2)),
            catalyst_unlocked BOOLEAN DEFAULT false,
            completed_at TIMESTAMP DEFAULT NOW(),
            created_at TIMESTAMP DEFAULT NOW()
        );"#,

        // User identities table
        r#"CREATE TABLE IF NOT EXISTS user_identities (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            user_id UUID UNIQUE REFERENCES users(id) ON DELETE CASCADE,
            nik VARCHAR(16) NOT NULL,
            full_name VARCHAR(255) NOT NULL,
            ktp_photo_url TEXT NOT NULL,
            selfie_url TEXT NOT NULL,
            is_verified BOOLEAN DEFAULT false,
            verified_at TIMESTAMP,
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Bank accounts table
        r#"CREATE TABLE IF NOT EXISTS bank_accounts (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            bank_code VARCHAR(20) NOT NULL,
            bank_name VARCHAR(100) NOT NULL,
            account_number VARCHAR(50) NOT NULL,
            account_name VARCHAR(255) NOT NULL,
            is_verified BOOLEAN DEFAULT false,
            is_primary BOOLEAN DEFAULT false,
            verified_at TIMESTAMP,
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Virtual accounts table
        r#"CREATE TABLE IF NOT EXISTS virtual_accounts (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            pool_id UUID REFERENCES funding_pools(id) ON DELETE CASCADE,
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            va_number VARCHAR(50) NOT NULL,
            bank_code VARCHAR(20) NOT NULL,
            bank_name VARCHAR(100) NOT NULL,
            amount DECIMAL(20,2) NOT NULL,
            status VARCHAR(20) DEFAULT 'pending' CHECK (status IN ('pending', 'paid', 'expired', 'cancelled')),
            expires_at TIMESTAMP NOT NULL,
            paid_at TIMESTAMP,
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Importer payments table
        r#"CREATE TABLE IF NOT EXISTS importer_payments (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            invoice_id UUID REFERENCES invoices(id) ON DELETE CASCADE NOT NULL,
            pool_id UUID REFERENCES funding_pools(id) NOT NULL,
            buyer_email VARCHAR(255) NOT NULL,
            buyer_name VARCHAR(255) NOT NULL,
            amount_due DECIMAL(20,2) NOT NULL,
            amount_paid DECIMAL(20,2) DEFAULT 0,
            currency VARCHAR(10) DEFAULT 'IDRX',
            payment_status VARCHAR(20) DEFAULT 'pending' CHECK (payment_status IN ('pending', 'paid', 'overdue', 'canceled')),
            due_date TIMESTAMP NOT NULL,
            paid_at TIMESTAMP,
            tx_hash VARCHAR(66),
            created_at TIMESTAMP DEFAULT NOW(),
            updated_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Country tiers table
        r#"CREATE TABLE IF NOT EXISTS country_tiers (
            country_code VARCHAR(3) PRIMARY KEY,
            country_name VARCHAR(100) NOT NULL,
            tier INTEGER NOT NULL CHECK (tier IN (1, 2, 3)),
            flag_emoji VARCHAR(10)
        );"#,

        // Balance transactions table
        r#"CREATE TABLE IF NOT EXISTS balance_transactions (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            type VARCHAR(30) NOT NULL CHECK (type IN ('deposit', 'withdrawal', 'funding', 'return', 'refund', 'disbursement')),
            amount DECIMAL(20,2) NOT NULL,
            balance_before DECIMAL(20,2),
            balance_after DECIMAL(20,2),
            reference_id UUID,
            reference_type VARCHAR(30),
            description TEXT,
            created_at TIMESTAMP DEFAULT NOW()
        );"#,

        // Indexes
        r#"CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_users_wallet ON users(wallet_address);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_invoices_exporter ON invoices(exporter_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_invoices_status ON invoices(status);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_invoices_due_date ON invoices(due_date);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_investments_pool ON investments(pool_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_investments_investor ON investments(investor_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_transactions_invoice ON transactions(invoice_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_transactions_user ON transactions(user_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_transactions_tx_hash ON transactions(tx_hash);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_nfts_token_id ON invoice_nfts(token_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_nfts_owner ON invoice_nfts(owner_address);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_otp_codes_email ON otp_codes(email);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_otp_codes_expires ON otp_codes(expires_at);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_mitra_applications_user ON mitra_applications(user_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_mitra_applications_status ON mitra_applications(status);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_risk_questionnaires_user ON risk_questionnaires(user_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_user_identities_user ON user_identities(user_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_user_identities_nik ON user_identities(nik);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_bank_accounts_user ON bank_accounts(user_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_bank_accounts_primary ON bank_accounts(user_id, is_primary);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_virtual_accounts_pool ON virtual_accounts(pool_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_virtual_accounts_user ON virtual_accounts(user_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_virtual_accounts_status ON virtual_accounts(status);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_importer_payments_invoice ON importer_payments(invoice_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_importer_payments_pool ON importer_payments(pool_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_importer_payments_status ON importer_payments(payment_status);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_balance_tx_user ON balance_transactions(user_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_balance_tx_type ON balance_transactions(type);"#,

        // Add explorer_url column to transactions for on-chain verification links
        r#"ALTER TABLE transactions ADD COLUMN IF NOT EXISTS explorer_url TEXT;"#,

        // Add wallet_nonce table for secure wallet authentication
        r#"CREATE TABLE IF NOT EXISTS wallet_nonces (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            wallet_address VARCHAR(42) NOT NULL,
            nonce VARCHAR(100) NOT NULL,
            expires_at TIMESTAMP NOT NULL,
            used BOOLEAN DEFAULT false,
            created_at TIMESTAMP DEFAULT NOW()
        );"#,

        r#"CREATE INDEX IF NOT EXISTS idx_wallet_nonces_address ON wallet_nonces(wallet_address);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_wallet_nonces_expires ON wallet_nonces(expires_at);"#,

        // Fix timestamp columns to use TIMESTAMPTZ for chrono compatibility
        r#"ALTER TABLE otp_codes ALTER COLUMN expires_at TYPE TIMESTAMPTZ USING expires_at AT TIME ZONE 'UTC';"#,
        r#"ALTER TABLE otp_codes ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'UTC';"#,
        r#"ALTER TABLE wallet_nonces ALTER COLUMN expires_at TYPE TIMESTAMPTZ USING expires_at AT TIME ZONE 'UTC';"#,
        r#"ALTER TABLE wallet_nonces ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'UTC';"#,

        // Insert default country tiers
        r#"INSERT INTO country_tiers (country_code, country_name, tier, flag_emoji) VALUES
            ('USA', 'United States', 1, '🇺🇸'),
            ('DEU', 'Germany', 1, '🇩🇪'),
            ('JPN', 'Japan', 1, '🇯🇵'),
            ('GBR', 'United Kingdom', 1, '🇬🇧'),
            ('FRA', 'France', 1, '🇫🇷'),
            ('CHE', 'Switzerland', 1, '🇨🇭'),
            ('NLD', 'Netherlands', 1, '🇳🇱'),
            ('AUS', 'Australia', 1, '🇦🇺'),
            ('CAN', 'Canada', 1, '🇨🇦'),
            ('SGP', 'Singapore', 1, '🇸🇬'),
            ('KOR', 'South Korea', 1, '🇰🇷'),
            ('CHN', 'China', 2, '🇨🇳'),
            ('IND', 'India', 2, '🇮🇳'),
            ('BRA', 'Brazil', 2, '🇧🇷'),
            ('MEX', 'Mexico', 2, '🇲🇽'),
            ('THA', 'Thailand', 2, '🇹🇭'),
            ('MYS', 'Malaysia', 2, '🇲🇾'),
            ('VNM', 'Vietnam', 2, '🇻🇳'),
            ('PHL', 'Philippines', 2, '🇵🇭'),
            ('IDN', 'Indonesia', 2, '🇮🇩'),
            ('TUR', 'Turkey', 2, '🇹🇷'),
            ('SAU', 'Saudi Arabia', 2, '🇸🇦'),
            ('ARE', 'UAE', 2, '🇦🇪'),
            ('NGA', 'Nigeria', 3, '🇳🇬'),
            ('PAK', 'Pakistan', 3, '🇵🇰'),
            ('BGD', 'Bangladesh', 3, '🇧🇩'),
            ('EGY', 'Egypt', 3, '🇪🇬'),
            ('KEN', 'Kenya', 3, '🇰🇪'),
            ('ZAF', 'South Africa', 3, '🇿🇦'),
            ('ARG', 'Argentina', 3, '🇦🇷')
        ON CONFLICT (country_code) DO NOTHING;"#,

        // Insert default admin user
        r#"INSERT INTO users (
            email, username, password_hash, role, is_verified, is_active,
            cooperative_agreement, member_status, balance_idr, email_verified, profile_completed
        ) VALUES (
            'admin@vessel.com',
            'admin',
            '$2a$10$4wj900d29YA0zv8R9PiBlOxvo5pJ94S90JsiY1PqX0IrW10NfKNIW',
            'admin',
            true,
            true,
            true,
            'admin',
            0,
            true,
            true
        ) ON CONFLICT (email) DO NOTHING;"#,
    ];

    for (i, migration) in migrations.iter().enumerate() {
        match sqlx::query(migration).execute(pool).await {
            Ok(_) => {}
            Err(e) => {
                warn!("Migration {} may have already been applied or failed: {}", i + 1, e);
            }
        }
    }

    info!("All migrations completed successfully");
    Ok(())
}
