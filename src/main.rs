use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::postgres::{PgPoolOptions, PgPool};
use dotenvy::dotenv;
use std::env;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

// Our registration handler
async fn register_user(
    State(pool): State<PgPool>, 
    Json(payload): Json<RegisterRequest>
) -> Result<impl IntoResponse, StatusCode> {
    
    // Generate a random salt and hash the password using Argon2
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = match argon2.hash_password(payload.password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Insert the new user into the database
    // The query! macro checks our SQL against the live database at compile time
    let result = sqlx::query!(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2)",
        payload.email,
        password_hash
    )
    .execute(&pool)
    .await;

    // Handle the database response
    match result {
        Ok(_) => Ok(StatusCode::CREATED),
        // If the email already exists, Postgres will return a unique violation error
        Err(_) => Err(StatusCode::CONFLICT), 
    }
}
#[tokio::main]
async fn main() {
    // Load environment variables from the .env file
    dotenv().ok();
    
    // Fetch the database URL
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    
    // Create a connection pool to PostgreSQL
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to Postgres. Is the database running?");
        
    println!("✅ Successfully connected to the database!");

    // Inject the database pool and add our new route
    let app = Router::new()
        .route("/", get(|| async { "Hello, IAM Platform! Connected to DB." }))
        .route("/auth/register", post(register_user)) // <-- New route here
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 Server running on http://localhost:3000");
    
    axum::serve(listener, app).await.unwrap();
}