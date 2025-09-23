use argon2::{Argon2, PasswordHasher, password_hash::{SaltString, PasswordHash, PasswordVerifier}};
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

pub fn hash_password(raw: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default().hash_password(raw.as_bytes(), &salt).map(|h| h.to_string()).map_err(|_| AuthError::Hash)
}

pub fn verify_password(raw: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed = PasswordHash::new(hash).map_err(|_| AuthError::Verify)?;
    Ok(Argon2::default().verify_password(raw.as_bytes(), &parsed).is_ok())
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
