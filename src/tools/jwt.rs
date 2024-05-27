use jsonwebtoken::{encode, Header, EncodingKey, decode, DecodingKey, Validation, Algorithm, TokenData};
use serde::{Serialize, Deserialize};
use chrono::{Utc, Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

// create jwt
pub async fn generate_jwt(username: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(8))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: username.to_owned(),
        exp: expiration as usize,
    };

    let sercet = "kenkone8282";
    encode(&Header::default(), &claims, &EncodingKey::from_secret(sercet.as_ref()))
}

// 驗證 JWT
pub fn validate_jwt(token: &str) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    let secret = "kenkone8282";
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::new(Algorithm::HS256),
    )
}