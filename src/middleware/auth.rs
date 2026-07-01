use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use std::env;
// We import our Claims struct from the models module
use crate::models::auth::Claims; 

pub async fn auth_middleware(
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .filter(|str| str.starts_with("Bearer "))
        .map(|str| str[7..].to_string());

    let token = auth_header.ok_or(StatusCode::UNAUTHORIZED)?;
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    
    let token_data = jsonwebtoken::decode::<Claims>(
        &token,
        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
        &jsonwebtoken::Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(token_data.claims);
    Ok(next.run(req).await)
}