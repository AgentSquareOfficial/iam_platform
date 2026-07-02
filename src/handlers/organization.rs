use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    auth::Claims,
    organization::{CreateOrganizationRequest, OrganizationResponse},
};

pub async fn create_organization(
    Extension(claims): Extension<Claims>,
    State(pool): State<PgPool>,
    Json(payload): Json<CreateOrganizationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut tx = pool.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let org = sqlx::query!(
        "INSERT INTO organizations (name) VALUES ($1) RETURNING id, name",
        payload.name
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query!(
        "INSERT INTO organization_memberships (user_id, organization_id) VALUES ($1, $2)",
        user_id,
        org.id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = OrganizationResponse {
        id: org.id,
        name: org.name,
    };

    Ok((StatusCode::CREATED, Json(response)))
}