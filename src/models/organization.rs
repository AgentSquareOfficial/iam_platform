use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct OrganizationResponse {
    pub id: uuid::Uuid,
    pub name: String,
}