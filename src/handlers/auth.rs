use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sqlx::PgPool;
use std::env;
use crate::models::auth::{AuthResponse, Claims, LoginRequest, RegisterRequest};

pub async fn register_user(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = match argon2.hash_password(payload.password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let result = sqlx::query!(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2)",
        payload.email,
        password_hash
    )
    .execute(&pool)
    .await;

    match result {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(_) => Err(StatusCode::CONFLICT),
    }
}

pub async fn login_user(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let user = sqlx::query!("SELECT id, password_hash FROM users WHERE email = $1", payload.email)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = match user {
        Some(u) => u,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !Argon2::default().verify_password(payload.password.as_bytes(), &parsed_hash).is_ok() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let expiration = chrono::Utc::now().checked_add_signed(chrono::Duration::hours(24)).unwrap().timestamp() as usize;

    let claims = Claims {
        sub: user.id.to_string(),
        exp: expiration,
    };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::OK, Json(AuthResponse { token })))
}

pub async fn get_me(
    Extension(claims): Extension<Claims>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let user_id = uuid::Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::BAD_REQUEST)?;

    let user = sqlx::query!("SELECT email, created_at FROM users WHERE id = $1", user_id)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match user {
        Some(u) => {
            let response = serde_json::json!({
                "id": claims.sub,
                "email": u.email,
                "created_at": u.created_at.to_string()
            });
            Ok((StatusCode::OK, Json(response)))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}