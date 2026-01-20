use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
    body::EitherBody,
};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use std::sync::Arc;

use crate::config::Config;
use crate::utils::{verify_token, Claims, ApiResponse};

pub struct AuthMiddleware {
    config: Arc<Config>,
}

impl AuthMiddleware {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddlewareService {
            service,
            config: self.config.clone(),
        })
    }
}

pub struct AuthMiddlewareService<S> {
    service: S,
    config: Arc<Config>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Extract token from Authorization header
        let auth_header = req.headers().get("Authorization");

        let token = auth_header
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "));

        match token {
            Some(token) => {
                match verify_token(token, &self.config.jwt_secret) {
                    Ok(claims) => {
                        // Insert claims into request extensions
                        req.extensions_mut().insert(claims);
                        let fut = self.service.call(req);
                        Box::pin(async move {
                            let res = fut.await?;
                            Ok(res.map_into_left_body())
                        })
                    }
                    Err(_) => {
                        let response = HttpResponse::Unauthorized()
                            .json(ApiResponse::<()>::error("Invalid or expired token"));
                        Box::pin(async move {
                            Ok(req.into_response(response).map_into_right_body())
                        })
                    }
                }
            }
            None => {
                let response = HttpResponse::Unauthorized()
                    .json(ApiResponse::<()>::error("Authorization header missing"));
                Box::pin(async move {
                    Ok(req.into_response(response).map_into_right_body())
                })
            }
        }
    }
}

// Optional auth middleware - doesn't fail if no token, just doesn't set claims
#[allow(dead_code)] // Reserved for future use
pub struct OptionalAuthMiddleware {
    config: Arc<Config>,
}

impl OptionalAuthMiddleware {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
}

impl<S, B> Transform<S, ServiceRequest> for OptionalAuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = OptionalAuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(OptionalAuthMiddlewareService {
            service,
            config: self.config.clone(),
        })
    }
}

#[allow(dead_code)] // Used by OptionalAuthMiddleware
pub struct OptionalAuthMiddlewareService<S> {
    service: S,
    config: Arc<Config>,
}

impl<S, B> Service<ServiceRequest> for OptionalAuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Try to extract token, but don't fail if not present
        if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(header_str) = auth_header.to_str() {
                if let Some(token) = header_str.strip_prefix("Bearer ") {
                    if let Ok(claims) = verify_token(token, &self.config.jwt_secret) {
                        req.extensions_mut().insert(claims);
                    }
                }
            }
        }

        let fut = self.service.call(req);
        Box::pin(async move { fut.await })
    }
}

// Admin-only middleware
pub struct AdminOnlyMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AdminOnlyMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AdminOnlyMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AdminOnlyMiddlewareService { service })
    }
}

pub struct AdminOnlyMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AdminOnlyMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let claims = req.extensions().get::<Claims>().cloned();

        match claims {
            Some(claims) if claims.role == "admin" => {
                let fut = self.service.call(req);
                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_left_body())
                })
            }
            Some(_) => {
                let response = HttpResponse::Forbidden()
                    .json(ApiResponse::<()>::error("Admin access required"));
                Box::pin(async move {
                    Ok(req.into_response(response).map_into_right_body())
                })
            }
            None => {
                let response = HttpResponse::Unauthorized()
                    .json(ApiResponse::<()>::error("Authentication required"));
                Box::pin(async move {
                    Ok(req.into_response(response).map_into_right_body())
                })
            }
        }
    }
}

// Mitra-only middleware
#[allow(dead_code)] // Reserved for future use
pub struct MitraOnlyMiddleware;

impl<S, B> Transform<S, ServiceRequest> for MitraOnlyMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = MitraOnlyMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(MitraOnlyMiddlewareService { service })
    }
}

#[allow(dead_code)] // Used by MitraOnlyMiddleware
pub struct MitraOnlyMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for MitraOnlyMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let claims = req.extensions().get::<Claims>().cloned();

        match claims {
            Some(claims) if claims.role == "mitra" || claims.role == "admin" => {
                let fut = self.service.call(req);
                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_left_body())
                })
            }
            Some(_) => {
                let response = HttpResponse::Forbidden()
                    .json(ApiResponse::<()>::error("Mitra access required"));
                Box::pin(async move {
                    Ok(req.into_response(response).map_into_right_body())
                })
            }
            None => {
                let response = HttpResponse::Unauthorized()
                    .json(ApiResponse::<()>::error("Authentication required"));
                Box::pin(async move {
                    Ok(req.into_response(response).map_into_right_body())
                })
            }
        }
    }
}
