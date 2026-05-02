use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Clerk User ID
    pub exp: usize,
}

pub struct AuthenticatedUser {
    pub user_id: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Get Authorization header
        let auth_header = parts.headers.get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header".into()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err((StatusCode::UNAUTHORIZED, "Invalid Authorization header".into()));
        }

        let token = &auth_header[7..];

        // 2. Validate JWT (Clerk)
        let pem = env::var("CLERK_PEM_PUBLIC_KEY").map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "CLERK_PEM_PUBLIC_KEY not set".into())
        })?;

        let decoding_key = DecodingKey::from_rsa_pem(pem.as_bytes()).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Invalid decoding key: {}", e))
        })?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        let token_data = decode::<Claims>(
            token,
            &decoding_key,
            &validation,
        ).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Invalid token: {}", e)))?;

        Ok(AuthenticatedUser {
            user_id: token_data.claims.sub,
        })
    }
}
