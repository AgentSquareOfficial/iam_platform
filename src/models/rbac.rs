use sqlx::PgPool;
use uuid::Uuid;

pub async fn has_permission(
    pool: &PgPool,
    user_id: Uuid,
    organization_id: Uuid,
    required_permission: &str,
) -> Result<bool, sqlx::Error> {
    
    // This query checks if there is a link between the user, their role in the org, 
    // and the specific permission node we are looking for.
    let result = sqlx::query!(
        r#"
        SELECT 1 AS has_access
        FROM member_roles mrc
        JOIN role_permissions rp ON mr.role_id = rp.role_id
        JOIN permissions p ON rp.permission_id = p.id
        WHERE mr.user_id = $1 
          AND mr.organization_id = $2 
          AND p.name = $3
        "#,
        user_id,
        organization_id,
        required_permission
    )
    .fetch_optional(pool)
    .await?;

    // If the query returns a row, they have permission. If it returns None, they don't.
    Ok(result.is_some())
}