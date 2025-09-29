use argon2::{
    password_hash::{PasswordHash, PasswordVerifier, SaltString},
    Argon2, PasswordHasher,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("hash error")]
    Hash,
    #[error("verify failed")]
    Verify,
    #[error("token encode error")]
    TokenEncode,
    #[error("token decode error")]
    TokenDecode,
}

// Tuned Argon2id parameters (balanced for security vs. latency; adjust after load tests)
const ARGON2_M_COST: u32 = 19456; // ~19 MB
const ARGON2_T_COST: u32 = 2; // iterations
const ARGON2_P_COST: u32 = 1; // parallelism (increase if CPU bound and acceptable)

fn argon2_instance() -> Argon2<'static> {
    // Build params via builder on Argon2 0.5 (if not available, fall back to default + comment)
    let params = argon2::Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, None)
        .expect("valid argon2 params");
    Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
}

pub fn hash_password(raw: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    argon2_instance()
        .hash_password(raw.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AuthError::Hash)
}

// Returns (is_valid, needs_rehash)
pub fn verify_password(raw: &str, hash: &str) -> Result<(bool, bool), AuthError> {
    let parsed = PasswordHash::new(hash).map_err(|_| AuthError::Verify)?;
    // Accept only Argon2id for future rehash decisions
    let alg_ok = parsed.algorithm.as_str() == argon2::Algorithm::Argon2id.as_ref();
    let valid = Argon2::default()
        .verify_password(raw.as_bytes(), &parsed)
        .is_ok();
    if !valid {
        return Ok((false, false));
    }
    // Determine if parameters differ from our target (trigger rehash on next login or background job)
    // Extract numeric params from the PHC string (m=, t=, p=)
    // Fallback parse of param string slice (e.g. "m=19456,t=2,p=1")
    let params_str = parsed.params.to_string();
    let mut m = None;
    let mut t = None;
    let mut p = None;
    for part in params_str.split(',') {
        if let Some((k, v)) = part.split_once('=') {
            if let Ok(num) = v.parse::<u32>() {
                match k {
                    "m" => m = Some(num),
                    "t" => t = Some(num),
                    "p" => p = Some(num),
                    _ => {}
                }
            }
        }
    }
    let needs_rehash = !alg_ok
        || m.unwrap_or(0) != ARGON2_M_COST
        || t.unwrap_or(0) != ARGON2_T_COST
        || p.unwrap_or(0) != ARGON2_P_COST;
    Ok((true, needs_rehash))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: u64,
    pub iss: String,
    pub iat: u64,
    pub typ: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

pub fn generate_tokens(
    user_id: &str,
    issuer: &str,
    secret: &str,
    access_ttl: Duration,
) -> Result<String, AuthError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let exp = now + access_ttl.as_secs();
    let claims = Claims {
        sub: user_id.to_string(),
        exp,
        iss: issuer.to_string(),
        iat: now,
        typ: "access".into(),
        email: None, // Can be added during token generation if needed
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AuthError::TokenEncode)
}

pub fn verify_jwt(token: &str, secret: &str, issuer: &str) -> Result<Claims, AuthError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[issuer]);
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AuthError::TokenDecode)?;
    Ok(data.claims)
}

pub fn decode_token(token: &str, secret: &str, issuer: &str) -> Result<Claims, AuthError> {
    verify_jwt(token, secret, issuer)
}
