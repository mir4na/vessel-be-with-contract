#![allow(dead_code)] // Many structs/methods are scaffolded for future features
#![allow(clippy::too_many_arguments)] // Suppress too many arguments lint globally

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod database;
mod error;
mod handlers;
mod middleware;
mod models;
mod repository;
mod services;
mod utils;

use config::Config;
use database::{create_pool, create_redis_pool};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");
    let config = Arc::new(config);

    info!("Starting VESSEL Backend on port {}", config.port);

    // Initialize database pool
    let db_pool = create_pool(&config)
        .await
        .expect("Failed to create database pool");

    // Run migrations
    database::run_migrations(&db_pool)
        .await
        .expect("Failed to run migrations");

    // Initialize Redis (optional)
    let redis_pool = match create_redis_pool(&config).await {
        Ok(pool) => {
            info!("Connected to Redis");
            Some(pool)
        }
        Err(e) => {
            tracing::warn!("Redis connection failed: {}. Continuing without Redis.", e);
            None
        }
    };

    // Initialize repositories
    let user_repo = Arc::new(repository::UserRepository::new(db_pool.clone()));
    let invoice_repo = Arc::new(repository::InvoiceRepository::new(db_pool.clone()));
    let funding_repo = Arc::new(repository::FundingRepository::new(db_pool.clone()));
    let tx_repo = Arc::new(repository::TransactionRepository::new(db_pool.clone()));
    let otp_repo = Arc::new(repository::OtpRepository::new(db_pool.clone()));
    let mitra_repo = Arc::new(repository::MitraRepository::new(db_pool.clone()));
    let importer_payment_repo =
        Arc::new(repository::ImporterPaymentRepository::new(db_pool.clone()));
    let rq_repo = Arc::new(repository::RiskQuestionnaireRepository::new(
        db_pool.clone(),
    ));

    // Initialize JWT Manager
    let jwt_manager = Arc::new(utils::JwtManager::new(
        &config.jwt_secret,
        config.jwt_expiry_hours,
        config.jwt_refresh_expiry_hours,
    ));

    // Initialize services
    let pinata_service = Arc::new(services::PinataService::new(config.clone()));
    let email_service = Arc::new(services::EmailService::new(config.clone()));
    let blockchain_service = Arc::new(
        services::BlockchainService::new(
            config.clone(),
            invoice_repo.clone(),
            funding_repo.clone(),
            pinata_service.clone(),
        )
        .await
        .expect("Failed to initialize blockchain service"),
    );
    let escrow_service = Arc::new(services::EscrowService::new());
    let otp_service = Arc::new(services::OtpService::new(
        otp_repo.clone(),
        email_service.clone(),
        config.clone(),
        jwt_manager.clone(),
    ));
    let auth_service = Arc::new(services::AuthService::new(
        user_repo.clone(),
        mitra_repo.clone(),
        jwt_manager.clone(),
        otp_service.clone(),
        config.clone(),
    ));
    let mitra_service = Arc::new(services::MitraService::new(
        mitra_repo.clone(),
        user_repo.clone(),
        email_service.clone(),
        pinata_service.clone(),
    ));
    let invoice_service = Arc::new(services::InvoiceService::new(
        invoice_repo.clone(),
        funding_repo.clone(),
        user_repo.clone(),
        mitra_repo.clone(),
        pinata_service.clone(),
        config.clone(),
    ));
    let funding_service = Arc::new(services::FundingService::new(
        funding_repo.clone(),
        invoice_repo.clone(),
        tx_repo.clone(),
        user_repo.clone(),
        rq_repo.clone(),
        email_service.clone(),
        escrow_service.clone(),
        blockchain_service.clone(),
        config.clone(),
    ));
    let payment_service = Arc::new(services::PaymentService::new(
        user_repo.clone(),
        tx_repo.clone(),
        funding_repo.clone(),
        invoice_repo.clone(),
        blockchain_service.clone(),
    ));
    let rq_service = Arc::new(services::RiskQuestionnaireService::new(rq_repo.clone()));
    let currency_service = Arc::new(services::CurrencyService::new(config.clone()));

    // Create application state
    let app_state = web::Data::new(handlers::AppState {
        config: config.clone(),
        db_pool: db_pool.clone(),
        redis_pool,
        jwt_manager: jwt_manager.clone(),
        user_repo: user_repo.clone(),
        invoice_repo,
        funding_repo: funding_repo.clone(),
        tx_repo: tx_repo.clone(),
        otp_repo,
        mitra_repo,
        importer_payment_repo,
        rq_repo,
        auth_service,
        otp_service,
        mitra_service,
        invoice_service,
        funding_service,
        payment_service,
        rq_service,
        currency_service,
        blockchain_service,
        pinata_service,
        email_service,
        escrow_service,
    });

    let server_port = config.port;
    let cors_origins = config.cors_allowed_origins.clone();

    HttpServer::new(move || {
        let cors_origins_inner = cors_origins.clone();
        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _req_head| {
                let origin_str = origin.to_str().unwrap_or("");
                if cors_origins_inner == "*" {
                    return true;
                }
                cors_origins_inner
                    .split(',')
                    .any(|o| o.trim() == origin_str)
            })
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec!["Authorization", "Content-Type", "Accept"])
            .supports_credentials()
            .max_age(3600);

        // Custom JSON error handler
        let json_cfg = web::JsonConfig::default().error_handler(|err, _req| {
            let message = format!("{}", err);
            actix_web::error::InternalError::from_response(
                err,
                actix_web::HttpResponse::BadRequest().json(serde_json::json!({
                    "success": false,
                    "error": {
                        "code": "VALIDATION_ERROR",
                        "message": message
                    }
                })),
            )
            .into()
        });

        App::new()
            .app_data(app_state.clone())
            .app_data(json_cfg)
            .wrap(Logger::default())
            .wrap(cors)
            // Health check
            .route("/health", web::get().to(handlers::health_check))
            // API v1 routes
            .service(
                web::scope("/api/v1")
                    // Auth routes (public)
                    .service(
                        web::scope("/auth")
                            // Traditional auth (for mitra/admin)
                            .route("/send-otp", web::post().to(handlers::auth::send_otp))
                            .route("/verify-otp", web::post().to(handlers::auth::verify_otp))
                            .route("/register", web::post().to(handlers::auth::register))
                            .route("/login", web::post().to(handlers::auth::login))
                            .route("/refresh", web::post().to(handlers::auth::refresh_token))
                            // Google OAuth (for mitra/admin - skips OTP)
                            .route("/google", web::post().to(handlers::auth::google_auth))
                            // Wallet auth (for investors)
                            .route(
                                "/wallet/nonce",
                                web::post().to(handlers::auth::get_wallet_nonce),
                            )
                            .route(
                                "/wallet/login",
                                web::post().to(handlers::auth::wallet_login),
                            )
                            .route(
                                "/wallet/register",
                                web::post().to(handlers::auth::wallet_register),
                            ),
                    )
                    // Public routes (for importers)
                    .service(
                        web::scope("/public")
                            .route(
                                "/payments/{payment_id}",
                                web::get().to(handlers::importer::get_payment_info),
                            )
                            .route(
                                "/payments/{payment_id}/pay",
                                web::post().to(handlers::importer::pay),
                            ),
                    )
                    // Protected routes
                    .service(
                        web::scope("")
                            .wrap(middleware::AuthMiddleware::new(config.clone()))
                            // User routes
                            .service(
                                web::scope("/user")
                                    .route("/profile", web::get().to(handlers::user::get_profile))
                                    .route(
                                        "/profile",
                                        web::put().to(handlers::user::update_profile),
                                    )
                                    .route(
                                        "/complete-profile",
                                        web::post().to(handlers::user::complete_profile),
                                    )
                                    .route(
                                        "/documents",
                                        web::post().to(handlers::user::upload_document),
                                    )
                                    .route(
                                        "/profile/data",
                                        web::get().to(handlers::user::get_personal_data),
                                    )
                                    .route(
                                        "/profile/password",
                                        web::put().to(handlers::user::change_password),
                                    )
                                    .route("/wallet", web::put().to(handlers::user::update_wallet))
                                    // Mitra application routes
                                    .service(
                                        web::scope("/mitra")
                                            .route("/apply", web::post().to(handlers::mitra::apply))
                                            .route(
                                                "/status",
                                                web::get().to(handlers::mitra::get_status),
                                            )
                                            .route(
                                                "/documents",
                                                web::post().to(handlers::mitra::upload_document),
                                            ),
                                    ),
                            )
                            // Currency routes
                            .service(
                                web::scope("/currency")
                                    .route(
                                        "/convert",
                                        web::post()
                                            .to(handlers::currency::get_locked_exchange_rate),
                                    )
                                    .route(
                                        "/supported",
                                        web::get().to(handlers::currency::get_supported_currencies),
                                    )
                                    .route(
                                        "/disbursement-estimate",
                                        web::get().to(
                                            handlers::currency::calculate_estimated_disbursement,
                                        ),
                                    ),
                            )
                            // Invoice routes
                            .service(
                                web::scope("/invoices")
                                    .route("", web::post().to(handlers::invoice::create))
                                    .route(
                                        "/funding-request",
                                        web::post().to(handlers::invoice::create_funding_request),
                                    )
                                    .route(
                                        "/check-repeat-buyer",
                                        web::post().to(handlers::invoice::check_repeat_buyer),
                                    )
                                    .route("", web::get().to(handlers::invoice::list))
                                    .route(
                                        "/fundable",
                                        web::get().to(handlers::invoice::list_fundable),
                                    )
                                    .route("/{id}", web::get().to(handlers::invoice::get))
                                    .route(
                                        "/{id}/detail",
                                        web::get().to(handlers::invoice::get_detail),
                                    )
                                    .route("/{id}", web::put().to(handlers::invoice::update))
                                    .route("/{id}", web::delete().to(handlers::invoice::delete))
                                    .route(
                                        "/{id}/submit",
                                        web::post().to(handlers::invoice::submit),
                                    )
                                    .route(
                                        "/{id}/documents",
                                        web::post().to(handlers::invoice::upload_document),
                                    )
                                    .route(
                                        "/{id}/documents",
                                        web::get().to(handlers::invoice::get_documents),
                                    )
                                    .route(
                                        "/{id}/tokenize",
                                        web::post().to(handlers::invoice::tokenize),
                                    )
                                    .route(
                                        "/{id}/pool",
                                        web::post().to(handlers::funding::create_pool),
                                    )
                                    .route(
                                        "/{id}/repay",
                                        web::post().to(handlers::funding::process_repayment),
                                    ),
                            )
                            // Pool routes
                            .service(
                                web::scope("/pools")
                                    .route("", web::get().to(handlers::funding::list_pools))
                                    .route("/{id}", web::get().to(handlers::funding::get_pool)),
                            )
                            // Marketplace routes
                            .service(
                                web::scope("/marketplace")
                                    .route("", web::get().to(handlers::funding::get_marketplace))
                                    .route(
                                        "/{id}/detail",
                                        web::get().to(handlers::funding::get_pool_detail),
                                    )
                                    .route(
                                        "/calculate",
                                        web::post().to(handlers::funding::calculate_investment),
                                    ),
                            )
                            // Risk questionnaire routes
                            .service(
                                web::scope("/risk-questionnaire")
                                    .route(
                                        "/questions",
                                        web::get().to(handlers::risk_questionnaire::get_questions),
                                    )
                                    .route("", web::post().to(handlers::risk_questionnaire::submit))
                                    .route(
                                        "/status",
                                        web::get().to(handlers::risk_questionnaire::get_status),
                                    ),
                            )
                            // Investment routes
                            .service(
                                web::scope("/investments")
                                    .route("", web::post().to(handlers::funding::invest))
                                    .route(
                                        "/confirm",
                                        web::post().to(handlers::funding::confirm_investment),
                                    )
                                    .route("", web::get().to(handlers::funding::get_my_investments))
                                    .route(
                                        "/portfolio",
                                        web::get().to(handlers::funding::get_portfolio),
                                    )
                                    .route(
                                        "/active",
                                        web::get().to(handlers::funding::get_active_investments),
                                    ),
                            )
                            // Exporter routes
                            .service(web::scope("/exporter").route(
                                "/disbursement",
                                web::post().to(handlers::funding::exporter_disbursement),
                            ))
                            // Mitra dashboard routes
                            .service(
                                web::scope("/mitra")
                                    .route(
                                        "/dashboard",
                                        web::get().to(handlers::funding::get_mitra_dashboard),
                                    )
                                    .route(
                                        "/pools",
                                        web::get().to(handlers::funding::get_mitra_pools),
                                    )
                                    .route(
                                        "/invoices",
                                        web::get().to(handlers::funding::get_mitra_active_invoices),
                                    )
                                    .route(
                                        "/invoices/active",
                                        web::get().to(handlers::mitra::get_active_invoices),
                                    )
                                    .route(
                                        "/invoices/{id}/pool",
                                        web::get().to(handlers::funding::get_pool_by_invoice),
                                    )
                                    .route(
                                        "/pools/{id}/breakdown",
                                        web::get().to(handlers::mitra::get_repayment_breakdown),
                                    ),
                            )
                            // Admin routes
                            .service(
                                web::scope("/admin")
                                    .wrap(middleware::AdminOnlyMiddleware)
                                    .route("/users", web::get().to(handlers::user::list_users))
                                    .route(
                                        "/invoices/pending",
                                        web::get().to(handlers::invoice::get_pending_invoices),
                                    )
                                    .route(
                                        "/invoices/approved",
                                        web::get().to(handlers::invoice::get_approved_invoices),
                                    )
                                    .route(
                                        "/invoices/{id}/grade-suggestion",
                                        web::get().to(handlers::invoice::get_grade_suggestion),
                                    )
                                    .route(
                                        "/invoices/{id}/review",
                                        web::get().to(handlers::invoice::get_invoice_review_data),
                                    )
                                    .route(
                                        "/invoices/{id}/approve",
                                        web::post().to(handlers::invoice::approve),
                                    )
                                    .route(
                                        "/invoices/{id}/reject",
                                        web::post().to(handlers::invoice::reject),
                                    )
                                    .route(
                                        "/users/{id}/invoices",
                                        web::get().to(handlers::invoice::get_exporter_invoices),
                                    )
                                    .route(
                                        "/users/{id}/pools",
                                        web::get().to(handlers::funding::get_exporter_pools),
                                    )
                                    .route(
                                        "/pools/{id}/disburse",
                                        web::post().to(handlers::funding::disburse),
                                    )
                                    .route(
                                        "/pools/{id}/close",
                                        web::post().to(handlers::funding::close_pool_and_notify),
                                    )
                                    .route(
                                        "/invoices/{id}/repay",
                                        web::post().to(handlers::funding::process_repayment),
                                    )
                                    .route(
                                        "/mitra/pending",
                                        web::get().to(handlers::mitra::get_pending_applications),
                                    )
                                    .route(
                                        "/mitra/all",
                                        web::get().to(handlers::mitra::get_all_applications),
                                    )
                                    .route(
                                        "/mitra/{id}",
                                        web::get().to(handlers::mitra::get_application),
                                    )
                                    .route(
                                        "/mitra/{id}/approve",
                                        web::post().to(handlers::mitra::approve),
                                    )
                                    .route(
                                        "/mitra/{id}/reject",
                                        web::post().to(handlers::mitra::reject),
                                    )
                                    .route(
                                        "/platform/revenue",
                                        web::get().to(handlers::payment::get_platform_revenue),
                                    ),
                            )
                            // Blockchain transparency routes (on-chain verification)
                            .service(
                                web::scope("/blockchain")
                                    // Public transparency endpoints (no auth needed for verification)
                                    .route(
                                        "/chain-info",
                                        web::get().to(handlers::blockchain::get_chain_info),
                                    )
                                    .route(
                                        "/balance/{address}",
                                        web::get().to(handlers::blockchain::get_idrx_balance),
                                    )
                                    .route(
                                        "/platform-balance",
                                        web::get().to(handlers::blockchain::get_platform_balance),
                                    )
                                    .route(
                                        "/verify/{tx_hash}",
                                        web::get().to(handlers::blockchain::verify_transaction),
                                    )
                                    .route(
                                        "/transfers/{address}",
                                        web::get().to(handlers::blockchain::get_transfer_history),
                                    )
                                    .route(
                                        "/pools/{id}/transactions",
                                        web::get().to(handlers::blockchain::get_pool_transactions),
                                    )
                                    // Authenticated endpoints
                                    .route(
                                        "/my-transactions",
                                        web::get().to(handlers::blockchain::get_my_transactions),
                                    )
                                    .route(
                                        "/my-idrx-balance",
                                        web::get().to(handlers::blockchain::get_my_idrx_balance),
                                    ),
                            ),
                    ),
            )
    })
    .bind(format!("0.0.0.0:{}", server_port))?
    .run()
    .await
}
