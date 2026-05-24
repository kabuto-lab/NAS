//! JWT verifier — HMAC SHA-256, shared secret with SITE1.
//!
//! Spec: `ADR-002 §D1` (shared HS256), `§D2` (claims structure).
//!
//! Verification flow:
//! 1. decode token with shared secret
//! 2. validate signature (HS256)
//! 3. validate exp (current time vs claims.exp)
//! 4. return Claims или mapped AppError
//!
//! No DB interaction — pure crypto + structure parse.

use ax_common::AppError;
use jsonwebtoken::{decode, errors::ErrorKind, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// JWT claims structure. Mirrors SITE1 NestJS `@nestjs/jwt` payload byte-for-byte
/// чтобы token signed на SITE1 верифицировался на AX без transformation.
///
/// Field naming snake_case (per SITE1 convention).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — user_id as string (Uuid)
    pub sub: String,
    /// Tenant binding — Uuid as string
    pub tenant_id: String,
    /// Role key — "admin" | "staff" | "client" — mirror SITE1
    pub role: String,
    /// Token kind — "platform" | "tenant" — mirror SITE1
    pub kind: String,
    /// Expiry timestamp (unix seconds)
    pub exp: i64,
    /// Issued-at timestamp (unix seconds)
    pub iat: i64,
}

/// JWT verifier — wraps jsonwebtoken decode_key + validation policy.
///
/// Thread-safe (read-only after construction). Suitable для Arc-share в AppState.
#[derive(Clone)]
pub struct JwtVerifier {
    decoding_key: DecodingKey,
    validation: Validation,
}

impl std::fmt::Debug for JwtVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtVerifier")
            .field("decoding_key", &"<redacted>")
            .field("validation_alg", &"HS256")
            .finish()
    }
}

impl JwtVerifier {
    /// Construct from shared secret string. Used at app startup with `JWT_SECRET` env.
    ///
    /// **CRITICAL:** secret должен быть same as SITE1 .env для cross-stack token compat.
    #[must_use]
    pub fn new(secret: &str) -> Self {
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.leeway = 0;
        // SITE1 NestJS uses default validators — exp + signature only. No aud/iss
        // enforcement at this layer (Phase B can add).
        validation.required_spec_claims = std::iter::once("exp".to_owned()).collect();
        Self {
            decoding_key,
            validation,
        }
    }

    /// Verify token + extract claims. Maps jsonwebtoken errors to AppError.
    ///
    /// # Errors
    /// - `AppError::TokenExpired` if exp в прошлом
    /// - `AppError::InvalidToken` for signature mismatch, malformed, decode failure, missing claims
    pub fn verify(&self, token: &str) -> Result<Claims, AppError> {
        match decode::<Claims>(token, &self.decoding_key, &self.validation) {
            Ok(data) => Ok(data.claims),
            Err(err) => Err(match err.kind() {
                ErrorKind::ExpiredSignature => AppError::TokenExpired,
                _ => AppError::InvalidToken,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    fn sign(secret: &str, claims: &Claims) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("sign")
    }

    fn claims(exp_offset: i64) -> Claims {
        let now = chrono::Utc::now().timestamp();
        Claims {
            sub: uuid::Uuid::new_v4().to_string(),
            tenant_id: uuid::Uuid::new_v4().to_string(),
            role: "admin".into(),
            kind: "tenant".into(),
            iat: now,
            exp: now + exp_offset,
        }
    }

    #[test]
    fn verify_valid_token_returns_claims() {
        let secret = "test-secret-min-32-chars-aaaaaaaa";
        let c = claims(60);
        let token = sign(secret, &c);
        let v = JwtVerifier::new(secret);
        let decoded = v.verify(&token).expect("verify");
        assert_eq!(decoded.sub, c.sub);
        assert_eq!(decoded.tenant_id, c.tenant_id);
        assert_eq!(decoded.role, "admin");
    }

    #[test]
    fn verify_expired_token_returns_token_expired() {
        let secret = "test-secret-min-32-chars-aaaaaaaa";
        let c = claims(-1); // expired 1s ago
        let token = sign(secret, &c);
        let v = JwtVerifier::new(secret);
        let err = v.verify(&token).expect_err("should fail");
        assert!(matches!(err, AppError::TokenExpired), "got {err:?}");
    }

    #[test]
    fn verify_wrong_secret_returns_invalid_token() {
        let c = claims(60);
        let token = sign("secret-a-aaaaaaaaaaaaaaaaaaaaaa", &c);
        let v = JwtVerifier::new("secret-b-bbbbbbbbbbbbbbbbbbbbbb");
        let err = v.verify(&token).expect_err("should fail");
        assert!(matches!(err, AppError::InvalidToken), "got {err:?}");
    }

    #[test]
    fn verify_malformed_token_returns_invalid_token() {
        let v = JwtVerifier::new("any-secret-aaaaaaaaaaaaaaaaaaaaa");
        let err = v.verify("not.a.token").expect_err("should fail");
        assert!(matches!(err, AppError::InvalidToken), "got {err:?}");
    }

    #[test]
    fn debug_redacts_key() {
        let v = JwtVerifier::new("super-secret-aaaaaaaaaaaaaaaaaa");
        let s = format!("{v:?}");
        assert!(!s.contains("super-secret"));
        assert!(s.contains("redacted"));
    }
}
