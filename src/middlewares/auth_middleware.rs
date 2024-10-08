use actix_web::{Error, dev::ServiceRequest, dev::ServiceResponse};
use actix_web::dev::{Transform, Service};
use actix_web::http::header;
use actix_web::{HttpResponse, body::BoxBody};
use futures::future::{ok, Ready};
// use futures::FutureExt;
use std::task::{Context, Poll};
use crate::services::auth_service::verify_jwt;

pub struct AuthMiddleware;

impl<S> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddlewareService { service })
    }
}

pub struct AuthMiddlewareService<S> {
    service: S,
}

impl<S> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = futures::future::Either<
        futures::future::Ready<Result<Self::Response, Self::Error>>,
        S::Future,
    >;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = auth_str.trim_start_matches("Bearer ");
                    if verify_jwt(token).is_ok() {
                        return futures::future::Either::Right(self.service.call(req));
                    }
                }
            }
        }

        futures::future::Either::Left(ok(req.into_response(
            HttpResponse::Unauthorized()
                .insert_header((header::WWW_AUTHENTICATE, "Bearer"))
                .finish()
                .map_into_boxed_body()  // Convert response body to BoxBody
        )))
    }
}
