# VESSEL Backend API Documentation

**Version:** 1.0
**Base URL:** `http://localhost:8080/api/v1`
**Authentication:** JWT Bearer Token

---

## Table of Contents

1. [Authentication](#1-authentication)
2. [User Management](#2-user-management)
3. [Invoice Management](#3-invoice-management)
4. [Admin Invoice Management](#4-admin-invoice-management)
5. [Funding Pool](#5-funding-pool)
6. [Investment](#6-investment)
7. [Payment](#7-payment)
8. [Mitra (Exporter)](#8-mitra-exporter)
9. [Currency & Exchange](#9-currency--exchange)
10. [Blockchain/Transparency](#10-blockchaintransparency)
11. [Risk Questionnaire](#11-risk-questionnaire)
12. [Importer Payment](#12-importer-payment)
13. [Admin User Management](#13-admin-user-management)

---

## Quick Setup

```bash
# Set base URL
export BASE_URL="http://localhost:8080/api/v1"

# After login, set your token
export TOKEN="your_jwt_token_here"
```

---

## Response Format

### Success Response
```json
{
  "success": true,
  "data": { },
  "message": "Descriptive message"
}
```

### Error Response
```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "Descriptive message"
  }
}
```

---

## 1. Authentication

**Base Path:** `/api/v1/auth`

### 1.1 Send OTP
Send OTP for email verification during registration or login.

```bash
# For registration
curl -X POST "$BASE_URL/auth/send-otp" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "purpose": "registration"
  }'

# For login
curl -X POST "$BASE_URL/auth/send-otp" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "purpose": "login"
  }'
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| email | string | Yes | User's email address |
| purpose | string | Yes | Either `registration` or `login` |

**Response:**
```json
{
  "success": true,
  "data": {
    "otp_token": "string",
    "expires_in_minutes": 10
  }
}
```

---

### 1.2 Verify OTP
Verify the OTP code sent to email.

```bash
curl -X POST "$BASE_URL/auth/verify-otp" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "otp": "123456"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "otp_token": "verified_token",
    "expires_in_minutes": 30
  }
}
```

---

### 1.3 Register (Mitra/Admin)
Register a new Mitra or Admin user.

```bash
curl -X POST "$BASE_URL/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "username": "username",
    "password": "securepassword",
    "confirm_password": "securepassword",
    "cooperative_agreement": true,
    "company_name": "PT Example",
    "company_type": "PT",
    "npwp": "12.345.678.9-012.345",
    "annual_revenue": 1000000000,
    "address": "Jakarta, Indonesia",
    "business_description": "Export business",
    "website_url": "https://example.com",
    "year_founded": 2020,
    "key_products": "Electronics",
    "export_markets": "USA, Europe"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "user": { },
    "access_token": "jwt_token",
    "refresh_token": "refresh_token",
    "expires_in": 86400
  }
}
```

---

### 1.4 Login (Mitra/Admin)
Login with email/username and password.

```bash
curl -X POST "$BASE_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email_or_username": "user@example.com",
    "password": "securepassword"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "user": { },
    "access_token": "jwt_token",
    "refresh_token": "refresh_token",
    "expires_in": 86400
  }
}
```

---

### 1.5 Refresh Token
Refresh the JWT access token.

```bash
curl -X POST "$BASE_URL/auth/refresh" \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "your_refresh_token"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "access_token": "new_jwt_token",
    "refresh_token": "new_refresh_token"
  }
}
```

---

### 1.6 Wallet Nonce (Investor)
Get nonce for wallet signature.

```bash
curl -X POST "$BASE_URL/auth/wallet/nonce" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "nonce": "random_nonce",
    "message": "Sign this message to login: random_nonce"
  }
}
```

---

### 1.7 Wallet Login (Investor)
Login with wallet signature.

```bash
curl -X POST "$BASE_URL/auth/wallet/login" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
    "signature": "0xsignature...",
    "message": "Sign this message to login: random_nonce",
    "nonce": "random_nonce",
    "tnc_accepted": true
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "user": { },
    "access_token": "jwt_token",
    "refresh_token": "refresh_token",
    "expires_in": 86400
  }
}
```

---

### 1.8 Wallet Register (Investor)
Register as investor with wallet.

```bash
curl -X POST "$BASE_URL/auth/wallet/register" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
    "signature": "0xsignature...",
    "message": "Sign this message to register: random_nonce",
    "nonce": "random_nonce",
    "cooperative_agreement": true
  }'
```

---

### 1.9 Google OAuth
Login with Google OAuth.

```bash
curl -X POST "$BASE_URL/auth/google" \
  -H "Content-Type: application/json" \
  -d '{
    "id_token": "google_id_token"
  }'
```

---

## 2. User Management

**Base Path:** `/api/v1/user`
**Authentication:** Required

### 2.1 Get Profile

```bash
curl -X GET "$BASE_URL/user/profile" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "email": "user@example.com",
    "username": "username",
    "role": "investor",
    "wallet_address": "0x...",
    "balance_idrx": "1000000",
    "profile_completed": true,
    "member_status": "active"
  }
}
```

---

### 2.2 Update Profile

```bash
curl -X PUT "$BASE_URL/user/profile" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "full_name": "John Doe",
    "phone": "+6281234567890",
    "country": "Indonesia",
    "company_name": "PT Example",
    "company_type": "PT",
    "business_sector": "Technology"
  }'
```

---

### 2.3 Complete Profile (KYC)

```bash
curl -X POST "$BASE_URL/user/complete-profile" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "full_name": "John Doe",
    "phone": "+6281234567890",
    "nik": "1234567890123456",
    "ktp_photo_url": "https://ipfs.io/...",
    "selfie_url": "https://ipfs.io/...",
    "bank_code": "BCA",
    "account_number": "1234567890",
    "account_name": "John Doe",
    "company_name": "PT Example",
    "country": "Indonesia"
  }'
```

**Supported Banks:** BCA, Mandiri, BNI, BRI, CIMB, Danamon, Permata, BSI, BTN, OCBC_NISP

---

### 2.4 Upload Document

```bash
curl -X POST "$BASE_URL/user/documents" \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@/path/to/document.pdf" \
  -F "document_type=ktp"
```

---

### 2.5 Get Full Profile Data

```bash
curl -X GET "$BASE_URL/user/profile/data" \
  -H "Authorization: Bearer $TOKEN"
```

---

### 2.6 Change Password

```bash
curl -X PUT "$BASE_URL/user/profile/password" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "oldpassword",
    "new_password": "newpassword",
    "confirm_password": "newpassword"
  }'
```

---

### 2.7 Update Wallet Address

```bash
curl -X PUT "$BASE_URL/user/wallet" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
  }'
```

---

## 3. Invoice Management

**Base Path:** `/api/v1/invoices`
**Authentication:** Required (Mitra role)

### 3.1 Create Invoice

```bash
curl -X POST "$BASE_URL/invoices" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "buyer_company_name": "ABC Corp",
    "buyer_country": "USA",
    "buyer_email": "buyer@abc.com",
    "invoice_number": "INV-2024-001",
    "original_currency": "USD",
    "original_amount": 10000,
    "locked_exchange_rate": 15500,
    "idr_amount": 155000000,
    "due_date": "2024-06-30",
    "funding_duration_days": 60,
    "priority_ratio": 0.8,
    "catalyst_ratio": 0.2,
    "priority_interest_rate": 8.5,
    "catalyst_interest_rate": 12.5,
    "is_repeat_buyer": false,
    "data_confirmation": true,
    "description": "Export electronics",
    "wallet_address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "uuid",
    "invoice_number": "INV-2024-001",
    "status": "draft",
    "amount": "155000000",
    "currency": "IDR"
  }
}
```

---

### 3.2 Create Funding Request

```bash
curl -X POST "$BASE_URL/invoices/funding-request" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "buyer_company_name": "ABC Corp",
    "buyer_country": "USA",
    "buyer_email": "buyer@abc.com",
    "invoice_number": "INV-2024-002",
    "original_currency": "USD",
    "original_amount": 10000,
    "locked_exchange_rate": 15500,
    "idr_amount": 155000000,
    "due_date": "2024-06-30",
    "funding_duration_days": 60,
    "priority_ratio": 0.8,
    "catalyst_ratio": 0.2,
    "priority_interest_rate": 8.5,
    "catalyst_interest_rate": 12.5,
    "is_repeat_buyer": false,
    "data_confirmation": true,
    "wallet_address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
  }'
```

---

### 3.3 Check Repeat Buyer

```bash
curl -X POST "$BASE_URL/invoices/check-repeat-buyer" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "buyer_company_name": "ABC Corp"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "is_repeat_buyer": true,
    "previous_transactions": 5,
    "funding_limit": 500000000
  }
}
```

---

### 3.4 List User's Invoices

```bash
# Basic
curl -X GET "$BASE_URL/invoices" \
  -H "Authorization: Bearer $TOKEN"

# With pagination and filter
curl -X GET "$BASE_URL/invoices?page=1&per_page=10&status=pending_review" \
  -H "Authorization: Bearer $TOKEN"
```

---

### 3.5 List Fundable Invoices

```bash
curl -X GET "$BASE_URL/invoices/fundable?page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

---

### 3.6 Get Invoice Details

```bash
curl -X GET "$BASE_URL/invoices/{invoice_id}" \
  -H "Authorization: Bearer $TOKEN"

# Example with actual ID
curl -X GET "$BASE_URL/invoices/550e8400-e29b-41d4-a716-446655440000" \
  -H "Authorization: Bearer $TOKEN"
```

---

### 3.7 Submit Invoice for Review

```bash
curl -X POST "$BASE_URL/invoices/{invoice_id}/submit" \
  -H "Authorization: Bearer $TOKEN"

# Example
curl -X POST "$BASE_URL/invoices/550e8400-e29b-41d4-a716-446655440000/submit" \
  -H "Authorization: Bearer $TOKEN"
```

---

### 3.8 Upload Invoice Document

```bash
curl -X POST "$BASE_URL/invoices/{invoice_id}/documents" \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@/path/to/invoice.pdf" \
  -F "document_type=invoice_pdf"

# Example with bill of lading
curl -X POST "$BASE_URL/invoices/550e8400-e29b-41d4-a716-446655440000/documents" \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@/path/to/bol.pdf" \
  -F "document_type=bill_of_lading"
```

**Document Types:** `invoice_pdf`, `bill_of_lading`, `packing_list`, `certificate_of_origin`, `insurance`, `customs`, `purchase_order`, `commercial_invoice`, `other`

---

### 3.9 Get Invoice Documents

```bash
curl -X GET "$BASE_URL/invoices/{invoice_id}/documents" \
  -H "Authorization: Bearer $TOKEN"
```

---

### Invoice Statuses
| Status | Description |
|--------|-------------|
| `draft` | Initial state, can be edited |
| `pending_review` | Submitted for admin review |
| `approved` | Approved by admin |
| `rejected` | Rejected by admin |
| `tokenized` | Converted to NFT |
| `funding` | Open for investment |
| `funded` | Fully funded |
| `matured` | Due date reached |
| `repaid` | Buyer has repaid |
| `defaulted` | Payment defaulted |

---

## 4. Admin Invoice Management

**Base Path:** `/api/v1/admin/invoices`
**Authentication:** Required (Admin role)

### 4.1 Get Pending Invoices

```bash
curl -X GET "$BASE_URL/admin/invoices/pending?page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

---

### 4.2 Get Approved Invoices

```bash
curl -X GET "$BASE_URL/admin/invoices/approved?page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

---

### 4.3 Get Grade Suggestion

```bash
curl -X GET "$BASE_URL/admin/invoices/{invoice_id}/grade-suggestion" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "suggested_grade": "A",
    "scoring": {
      "buyer_history": 25,
      "documentation": 20,
      "country_risk": 15,
      "invoice_amount": 10,
      "total": 70
    }
  }
}
```

---

### 4.4 Get Review Data

```bash
curl -X GET "$BASE_URL/admin/invoices/{invoice_id}/review" \
  -H "Authorization: Bearer $TOKEN"
```

---

### 4.5 Approve Invoice

```bash
curl -X POST "$BASE_URL/admin/invoices/{invoice_id}/approve" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "grade": "A",
    "priority_interest_rate": 8.5,
    "catalyst_interest_rate": 12.5,
    "notes": "Approved with standard terms"
  }'
```

---

### 4.6 Reject Invoice

```bash
curl -X POST "$BASE_URL/admin/invoices/{invoice_id}/reject" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "Insufficient documentation"
  }'
```

---

## 5. Funding Pool

**Base Path:** `/api/v1`
**Authentication:** Required

### 5.1 Create Funding Pool

```bash
curl -X POST "$BASE_URL/invoices/{invoice_id}/pool" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "funding_deadline_hours": 72
  }'
```

---

### 5.2 List Pools

```bash
# All pools
curl -X GET "$BASE_URL/pools" \
  -H "Authorization: Bearer $TOKEN"

# With filters
curl -X GET "$BASE_URL/pools?page=1&per_page=10&status=open" \
  -H "Authorization: Bearer $TOKEN"
```

**Pool Statuses:** `open`, `filled`, `disbursed`, `closed`

---

### 5.3 Get Pool Details

```bash
curl -X GET "$BASE_URL/pools/{pool_id}" \
  -H "Authorization: Bearer $TOKEN"
```

---

### 5.4 Get Marketplace Pools

```bash
curl -X GET "$BASE_URL/marketplace?page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "uuid",
      "invoice_number": "INV-2024-001",
      "buyer_name": "ABC Corp",
      "buyer_country": "USA",
      "target_amount": "155000000",
      "funded_amount": "100000000",
      "progress_percentage": 64.5,
      "priority_available": "24000000",
      "catalyst_available": "31000000",
      "priority_interest_rate": 8.5,
      "catalyst_interest_rate": 12.5,
      "deadline": "2024-03-15T00:00:00Z",
      "grade": "A"
    }
  ]
}
```

---

### 5.5 Get Pool Marketplace Detail

```bash
curl -X GET "$BASE_URL/marketplace/{pool_id}/detail" \
  -H "Authorization: Bearer $TOKEN"
```

---

### 5.6 Calculate Investment Returns

```bash
curl -X POST "$BASE_URL/marketplace/calculate" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "pool_id": "550e8400-e29b-41d4-a716-446655440000",
    "amount": 10000000,
    "tranche": "priority"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "principal": "10000000",
    "interest_rate": 8.5,
    "expected_return": "850000",
    "total_return": "10850000",
    "duration_days": 60
  }
}
```

---

## 6. Investment

**Base Path:** `/api/v1/investments`
**Authentication:** Required (Investor role)

### 6.1 Create Investment

```bash
curl -X POST "$BASE_URL/investments" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "pool_id": "550e8400-e29b-41d4-a716-446655440000",
    "amount": 10000000,
    "tranche": "priority",
    "tx_hash": "0x1234567890abcdef...",
    "tnc_accepted": true
  }'

# For catalyst tranche (requires risk questionnaire)
curl -X POST "$BASE_URL/investments" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "pool_id": "550e8400-e29b-41d4-a716-446655440000",
    "amount": 5000000,
    "tranche": "catalyst",
    "tx_hash": "0x1234567890abcdef...",
    "tnc_accepted": true,
    "catalyst_consents": {
      "risk_acknowledged": true,
      "loss_potential_understood": true
    }
  }'
```

**Tranche Types:**
- `priority`: Lower risk, lower yield (paid first)
- `catalyst`: Higher risk, higher yield (paid after priority)

---

### 6.2 Confirm Investment

```bash
curl -X POST "$BASE_URL/investments/confirm" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "investment_id": "550e8400-e29b-41d4-a716-446655440000"
  }'
```

---

### 6.3 Get User's Investments

```bash
curl -X GET "$BASE_URL/investments?page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

---

### 6.4 Get Portfolio Summary

```bash
curl -X GET "$BASE_URL/investments/portfolio" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "total_invested": "100000000",
    "total_returns": "8500000",
    "active_investments": 5,
    "completed_investments": 10,
    "average_yield": 8.5,
    "portfolio_by_tranche": {
      "priority": "70000000",
      "catalyst": "30000000"
    }
  }
}
```

---

### 6.5 Get Active Investments

```bash
curl -X GET "$BASE_URL/investments/active?page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

---

## 7. Payment

**Base Path:** `/api/v1/payments`
**Authentication:** Required

### 7.1 Get IDRX Balance

```bash
curl -X GET "$BASE_URL/payments/balance" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "balance_idrx": "50000000",
    "currency": "IDRX"
  }
}
```

---

### 7.2 Deposit IDRX

```bash
curl -X POST "$BASE_URL/payments/deposit" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": 10000000,
    "tx_hash": "0x1234567890abcdef..."
  }'
```

---

### 7.3 Withdraw IDRX

```bash
curl -X POST "$BASE_URL/payments/withdraw" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": 5000000,
    "to_address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
  }'
```

---

### 7.4 Get Platform Revenue (Admin Only)

```bash
curl -X GET "$BASE_URL/admin/platform/revenue" \
  -H "Authorization: Bearer $TOKEN"
```

---

## 8. Mitra (Exporter)

### 8.1 User Mitra Endpoints

**Base Path:** `/api/v1/user/mitra`

#### Apply to be Mitra

```bash
curl -X POST "$BASE_URL/user/mitra/apply" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "company_name": "PT Example Export",
    "company_type": "PT",
    "npwp": "12.345.678.9-012.345",
    "annual_revenue": 5000000000,
    "address": "Jakarta, Indonesia",
    "business_description": "Export textiles",
    "website_url": "https://example.com",
    "year_founded": 2015,
    "key_products": "Textiles, Garments",
    "export_markets": "USA, Europe, Japan"
  }'
```

---

#### Get Application Status

```bash
curl -X GET "$BASE_URL/user/mitra/status" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "status": "pending",
    "documents_status": {
      "nib": "uploaded",
      "akta_pendirian": "uploaded",
      "ktp_direktur": "pending"
    }
  }
}
```

---

#### Upload Mitra Document

```bash
# Upload NIB
curl -X POST "$BASE_URL/user/mitra/documents" \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@/path/to/nib.pdf" \
  -F "document_type=nib"

# Upload Akta Pendirian
curl -X POST "$BASE_URL/user/mitra/documents" \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@/path/to/akta.pdf" \
  -F "document_type=akta_pendirian"

# Upload KTP Direktur
curl -X POST "$BASE_URL/user/mitra/documents" \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@/path/to/ktp.jpg" \
  -F "document_type=ktp_direktur"
```

---

### 8.2 Admin Mitra Endpoints

**Base Path:** `/api/v1/admin/mitra`

#### Get Pending Applications

```bash
curl -X GET "$BASE_URL/admin/mitra/pending?page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

---

#### Get All Applications

```bash
curl -X GET "$BASE_URL/admin/mitra/all?page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

---

#### Get Single Application

```bash
curl -X GET "$BASE_URL/admin/mitra/{application_id}" \
  -H "Authorization: Bearer $TOKEN"
```

---

#### Approve Application

```bash
curl -X POST "$BASE_URL/admin/mitra/{application_id}/approve" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "notes": "All documents verified"
  }'
```

---

#### Reject Application

```bash
curl -X POST "$BASE_URL/admin/mitra/{application_id}/reject" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "Invalid NPWP"
  }'
```

---

### 8.3 Mitra Dashboard Endpoints

**Base Path:** `/api/v1/mitra`

#### Get Dashboard

```bash
curl -X GET "$BASE_URL/mitra/dashboard" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "total_invoices": 10,
    "active_invoices": 3,
    "total_funded": "500000000",
    "pending_repayment": "155000000",
    "active_invoices_list": []
  }
}
```

---

#### Get Active Invoices

```bash
curl -X GET "$BASE_URL/mitra/invoices/active?page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

---

#### Get Mitra Invoices

Get all invoices owned by the authenticated mitra user.

```bash
curl -X GET "$BASE_URL/mitra/invoices?page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "uuid",
      "invoice_number": "INV-001",
      "buyer_name": "PT Buyer",
      "buyer_country": "Singapore",
      "amount": "10000.00",
      "currency": "IDR",
      "status": "funding",
      "due_date": "2026-02-02",
      "created_at": "2026-01-25T17:48:09.444536"
    }
  ],
  "pagination": {
    "page": 1,
    "per_page": 10,
    "total": 5,
    "total_pages": 1
  }
}
```

---

#### Get Mitra Pools

Get all funding pools for invoices owned by the authenticated mitra user.

```bash
curl -X GET "$BASE_URL/mitra/pools?page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "pool": {
        "id": "uuid",
        "invoice_id": "uuid",
        "target_amount": "10000.00",
        "funded_amount": "5000.00",
        "investor_count": 2,
        "status": "open",
        "priority_target": "8000.00",
        "priority_funded": "5000.00",
        "catalyst_target": "2000.00",
        "catalyst_funded": "0",
        "priority_interest_rate": "10.00",
        "catalyst_interest_rate": "15.00",
        "deadline": "2026-02-08T18:06:29.232264"
      },
      "remaining_amount": 5000.0,
      "percentage_funded": 50.0,
      "priority_remaining": 3000.0,
      "catalyst_remaining": 2000.0,
      "priority_percentage_funded": 62.5,
      "catalyst_percentage_funded": 0.0,
      "invoice": {
        "id": "uuid",
        "invoice_number": "INV-001",
        "buyer_name": "PT Buyer",
        "status": "funding"
      }
    }
  ],
  "pagination": {
    "page": 1,
    "per_page": 10,
    "total": 2,
    "total_pages": 1
  }
}
```

---

#### Get Pool by Invoice ID

Get the funding pool details for a specific invoice owned by the authenticated mitra user.

```bash
curl -X GET "$BASE_URL/mitra/invoices/{invoice_id}/pool" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "message": "Pool detail retrieved",
  "data": {
    "pool": {
      "id": "uuid",
      "invoice_id": "uuid",
      "target_amount": "10000.00",
      "funded_amount": "5000.00",
      "status": "open"
    },
    "remaining_amount": 5000.0,
    "percentage_funded": 50.0,
    "invoice": { }
  }
}
```

**Error Responses:**
- `404 NOT_FOUND`: Invoice not found
- `403 FORBIDDEN`: Not the invoice owner (invoice belongs to another user)
- `404 NOT_FOUND`: No funding pool found for this invoice

---

#### Get Repayment Breakdown

```bash
curl -X GET "$BASE_URL/mitra/pools/{pool_id}/breakdown" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "pool_id": "uuid",
    "total_amount_due": "165000000",
    "principal": "155000000",
    "interest": "8500000",
    "platform_fee": "1500000",
    "due_date": "2024-06-30",
    "breakdown_by_tranche": {
      "priority": {
        "principal": "124000000",
        "interest": "5270000"
      },
      "catalyst": {
        "principal": "31000000",
        "interest": "3230000"
      }
    }
  }
}
```

---

#### Process Repayment

```bash
curl -X POST "$BASE_URL/mitra/invoices/{invoice_id}/repay" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": 165000000,
    "tx_hash": "0x1234567890abcdef...",
    "notes": "Full repayment"
  }'
```

---

#### Request Disbursement

```bash
curl -X POST "$BASE_URL/exporter/disbursement" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "pool_id": "550e8400-e29b-41d4-a716-446655440000",
    "bank_account_id": "550e8400-e29b-41d4-a716-446655440001"
  }'
```

---

## 9. Currency & Exchange

**Base Path:** `/api/v1/currency`

### 9.1 Get Supported Currencies

```bash
curl -X GET "$BASE_URL/currency/supported"
```

**Response:**
```json
{
  "success": true,
  "data": [
    { "code": "USD", "name": "US Dollar", "symbol": "$", "flag_emoji": "🇺🇸" },
    { "code": "EUR", "name": "Euro", "symbol": "€", "flag_emoji": "🇪🇺" },
    { "code": "GBP", "name": "British Pound", "symbol": "£", "flag_emoji": "🇬🇧" },
    { "code": "JPY", "name": "Japanese Yen", "symbol": "¥", "flag_emoji": "🇯🇵" },
    { "code": "SGD", "name": "Singapore Dollar", "symbol": "S$", "flag_emoji": "🇸🇬" },
    { "code": "AUD", "name": "Australian Dollar", "symbol": "A$", "flag_emoji": "🇦🇺" },
    { "code": "CNY", "name": "Chinese Yuan", "symbol": "¥", "flag_emoji": "🇨🇳" }
  ]
}
```

---

### 9.2 Convert Currency (Get Locked Rate)

```bash
curl -X POST "$BASE_URL/currency/convert" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "from_currency": "USD",
    "amount": 10000
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "from_currency": "USD",
    "to_currency": "IDR",
    "original_amount": "10000",
    "exchange_rate": "15500",
    "buffer_rate": "0.02",
    "effective_rate": "15190",
    "converted_amount": "151900000",
    "locked_until": "2024-03-15T12:00:00Z",
    "rate_lock_token": "token_string"
  }
}
```

---

### 9.3 Get Disbursement Estimate

```bash
curl -X GET "$BASE_URL/currency/disbursement-estimate?amount=155000000" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "gross_amount": "155000000",
    "platform_fee_percentage": 2.5,
    "platform_fee_amount": "3875000",
    "net_disbursement": "151125000",
    "currency": "IDRX"
  }
}
```

---

## 10. Blockchain/Transparency

**Base Path:** `/api/v1/blockchain`

### 10.1 Public Endpoints (No Auth Required)

#### Get Chain Info

```bash
curl -X GET "$BASE_URL/blockchain/chain-info"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "chain_id": 31337,
    "chain_name": "Hardhat Local",
    "current_block": 12345,
    "rpc_url": "http://127.0.0.1:8545",
    "explorer_url": "http://localhost:8545",
    "idrx_contract": "0x5FbDB2315678afecb367f032d93F642f64180aa3",
    "platform_wallet": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
  }
}
```

---

#### Get Balance by Address

```bash
curl -X GET "$BASE_URL/blockchain/balance/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "address": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
    "balance": "1000000000",
    "currency": "IDRX",
    "chain": "Hardhat Local",
    "chain_id": 31337,
    "explorer_url": "http://localhost:8545"
  }
}
```

---

#### Get Platform Balance

```bash
curl -X GET "$BASE_URL/blockchain/platform-balance"
```

---

#### Verify Transaction

```bash
curl -X GET "$BASE_URL/blockchain/verify/0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "tx_hash": "0x1234...",
    "verified": true,
    "block_number": 12345,
    "chain": "Hardhat Local",
    "chain_id": 31337,
    "explorer_url": "http://localhost:8545/tx/0x1234..."
  }
}
```

---

#### Get Transfer History

```bash
curl -X GET "$BASE_URL/blockchain/transfers/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
```

---

#### Get Pool Transactions

```bash
curl -X GET "$BASE_URL/blockchain/pools/{pool_id}/transactions"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "pool_id": "uuid",
    "invoice_id": "uuid",
    "status": "funded",
    "transactions": [
      {
        "tx_hash": "0x...",
        "type": "investment",
        "amount": "10000000",
        "from": "0x...",
        "to": "0x...",
        "block_number": 12345,
        "timestamp": "2024-03-10T10:00:00Z"
      }
    ],
    "total_invested_on_chain": "155000000"
  }
}
```

---

### 10.2 Authenticated Endpoints

#### Get My Transactions

```bash
curl -X GET "$BASE_URL/blockchain/my-transactions?page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

---

#### Get My IDRX Balance

```bash
curl -X GET "$BASE_URL/blockchain/my-idrx-balance" \
  -H "Authorization: Bearer $TOKEN"
```

---

## 11. Risk Questionnaire

**Base Path:** `/api/v1/risk-questionnaire`
**Authentication:** Required (Investor role)

### 11.1 Get Questions

```bash
curl -X GET "$BASE_URL/risk-questionnaire/questions" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "q1",
      "question": "What is your investment experience?",
      "options": [
        { "value": "a", "label": "No experience" },
        { "value": "b", "label": "1-3 years" },
        { "value": "c", "label": "3+ years" }
      ],
      "required_for_catalyst": true
    }
  ]
}
```

---

### 11.2 Submit Questionnaire

```bash
curl -X POST "$BASE_URL/risk-questionnaire" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "q1_answer": "c",
    "q2_answer": "b",
    "q3_answer": "a"
  }'
```

---

### 11.3 Get Status

```bash
curl -X GET "$BASE_URL/risk-questionnaire/status" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "completed": true,
    "catalyst_unlocked": true,
    "completed_at": "2024-03-10T10:00:00Z",
    "answers": {
      "q1": "c",
      "q2": "b",
      "q3": "a"
    }
  }
}
```

---

## 12. Importer Payment

**Base Path:** `/api/v1/public/payments`
**Authentication:** Not Required

### 12.1 Get Payment Info

```bash
curl -X GET "$BASE_URL/public/payments/{payment_id}"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "payment_id": "uuid",
    "invoice_number": "INV-2024-001",
    "amount_due": "165000000",
    "currency": "IDRX",
    "exporter_name": "PT Example",
    "due_date": "2024-06-30",
    "status": "pending"
  }
}
```

---

### 12.2 Submit Payment

```bash
curl -X POST "$BASE_URL/public/payments/{payment_id}/pay" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": 165000000,
    "tx_hash": "0x1234567890abcdef..."
  }'
```

---

## 13. Admin User Management

**Base Path:** `/api/v1/admin/users`
**Authentication:** Required (Admin role)

### 13.1 List All Users

```bash
# All users
curl -X GET "$BASE_URL/admin/users" \
  -H "Authorization: Bearer $TOKEN"

# Filter by role
curl -X GET "$BASE_URL/admin/users?role=investor&page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"

# Filter mitra users
curl -X GET "$BASE_URL/admin/users?role=mitra&page=1&per_page=10" \
  -H "Authorization: Bearer $TOKEN"
```

**Query Parameters:**
- `role`: Filter by role (`investor`, `mitra`, `admin`, `exporter`)
- `page`: Page number
- `per_page`: Items per page

---

## Quick Test Scripts

### Test Authentication Flow

```bash
#!/bin/bash
export BASE_URL="http://localhost:8080/api/v1"

# 1. Send OTP
echo "=== Sending OTP ==="
curl -s -X POST "$BASE_URL/auth/send-otp" \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com"}' | jq

# 2. Login (if already registered)
echo "=== Login ==="
RESPONSE=$(curl -s -X POST "$BASE_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email_or_username": "test@example.com", "password": "password123"}')
echo $RESPONSE | jq

# Extract token
export TOKEN=$(echo $RESPONSE | jq -r '.data.access_token')
echo "Token: $TOKEN"

# 3. Get Profile
echo "=== Get Profile ==="
curl -s -X GET "$BASE_URL/user/profile" \
  -H "Authorization: Bearer $TOKEN" | jq
```

---

### Test Blockchain Endpoints

```bash
#!/bin/bash
export BASE_URL="http://localhost:8080/api/v1"

# Chain Info (no auth needed)
echo "=== Chain Info ==="
curl -s -X GET "$BASE_URL/blockchain/chain-info" | jq

# Platform Balance
echo "=== Platform Balance ==="
curl -s -X GET "$BASE_URL/blockchain/platform-balance" | jq

# Check specific address balance
echo "=== Address Balance ==="
curl -s -X GET "$BASE_URL/blockchain/balance/0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266" | jq
```

---

### Test Currency Endpoints

```bash
#!/bin/bash
export BASE_URL="http://localhost:8080/api/v1"

# Get supported currencies
echo "=== Supported Currencies ==="
curl -s -X GET "$BASE_URL/currency/supported" | jq

# Convert USD to IDR
echo "=== Convert USD to IDR ==="
curl -s -X POST "$BASE_URL/currency/convert" \
  -H "Content-Type: application/json" \
  -d '{"from_currency": "USD", "amount": 10000}' | jq
```

---

## Error Codes

| Code | Description |
|------|-------------|
| `VALIDATION_ERROR` | Invalid input data |
| `UNAUTHORIZED` | Missing or invalid authentication |
| `FORBIDDEN` | Insufficient permissions |
| `NOT_FOUND` | Resource not found |
| `CONFLICT` | Resource already exists |
| `INTERNAL_ERROR` | Server error |

---

## Rate Limiting

- General API: 100 requests/minute
- Auth endpoints: 10 requests/minute

---

## Smart Contract Addresses (Localhost)

| Contract | Address |
|----------|---------|
| IDRX Token | `0x5FbDB2315678afecb367f032d93F642f64180aa3` |
| Invoice NFT | `0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512` |
| Funding Pool | `0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0` |

---

## Blockchain Configuration

| Setting | Value |
|---------|-------|
| RPC URL | `http://127.0.0.1:8545` |
| Chain ID | `31337` |
| Platform Wallet | `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` |

---

## Hardhat Test Accounts

For local testing, you can use these pre-funded accounts:

| Account | Address | Private Key |
|---------|---------|-------------|
| #0 | `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266` | `0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80` |
| #1 | `0x70997970C51812dc3A010C7d01b50e0d17dc79C8` | `0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d` |
| #2 | `0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC` | `0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a` |

**WARNING:** These are publicly known test accounts. Never use them on mainnet!
