use argon2::{Argon2, PasswordHasher, password_hash::{SaltString, PasswordHash, PasswordVerifier, Ident, Algorithm as PHAlgorithm, Params as PHParams}};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm};
use rand::rngs::OsRng;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use std::time::{SystemTime, Duration, UNIX_EPOCH};

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("hash error")] Hash,
    #[error("verify failed")] Verify,
    #[error("token encode error")] TokenEncode,
    #[error("token decode error")] TokenDecode,
}

// Tuned Argon2id parameters (balanced for security vs. latency; adjust after load tests)
const ARGON2_M_COST: u32 = 19456; // ~19 MB
const ARGON2_T_COST: u32 = 2;     // iterations
const ARGON2_P_COST: u32 = 1;     // parallelism (increase if CPU bound and acceptable)

fn argon2_instance() -> Argon2 {
    let params = PHParams::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, None)
        .expect("valid argon2 params");
    Argon2::from_phf(Ident::Argon2id, PHAlgorithm::Argon2id, params)
}

pub fn hash_password(raw: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    argon2_instance().hash_password(raw.as_bytes(), &salt).map(|h| h.to_string()).map_err(|_| AuthError::Hash)
}

// Returns (is_valid, needs_rehash)
pub fn verify_password(raw: &str, hash: &str) -> Result<(bool, bool), AuthError> {
    let parsed = PasswordHash::new(hash).map_err(|_| AuthError::Verify)?;
    let alg_ok = parsed.algorithm == PHAlgorithm::Argon2id.ident();
    let valid = Argon2::default().verify_password(raw.as_bytes(), &parsed).is_ok();
    if !valid { return Ok((false, false)); }
    // Determine if parameters differ from our target (trigger rehash on next login or background job)
    let needs_rehash = !alg_ok || parsed.params.get("m").and_then(|v| v.decimal()).unwrap_or(0) != ARGON2_M_COST as u64
        || parsed.params.get("t").and_then(|v| v.decimal()).unwrap_or(0) != ARGON2_T_COST as u64
        || parsed.params.get("p").and_then(|v| v.decimal()).unwrap_or(0) != ARGON2_P_COST as u64;
    Ok((true, needs_rehash))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims { pub sub: String, pub exp: u64, pub iss: String, pub iat: u64, pub typ: String }

pub fn generate_tokens(user_id: &str, issuer: &str, secret: &str, access_ttl: Duration) -> Result<String, AuthError> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let exp = now + access_ttl.as_secs();
    let claims = Claims { sub: user_id.to_string(), exp, iss: issuer.to_string(), iat: now, typ: "access".into() };
    encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret.as_bytes())).map_err(|_| AuthError::TokenEncode)
}

pub fn decode_token(token: &str, secret: &str, issuer: &str) -> Result<Claims, AuthError> {
    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::new(Algorithm::HS256)).map_err(|_| AuthError::TokenDecode)?;
    if data.claims.iss != issuer { return Err(AuthError::TokenDecode); }
    Ok(data.claims)
}
