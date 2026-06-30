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
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

// We use Serialize here so Axum can convert this struct into JSON for the response
#[derive(serde::Serialize)]
pub struct AuthResponse {
    pub token: String,
}

// "Claims" are the pieces of information stored inside the JWT
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (Usually the User's ID)
    pub exp: usize,  // Expiration time
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

async fn login_user(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    
    // 1. Fetch the user from the database by email
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // If the user doesn't exist, return 401 Unauthorized
    let user = match user {
        Some(u) => u,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // 2. Parse the stored hash and verify the provided password against it
    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let is_valid = Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_ok();

    if !is_valid {
        return Err(StatusCode::UNAUTHORIZED); // Wrong password
    }

    // 3. Setup JWT Configuration
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env");
    
    // Set the token to expire in 24 hours
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user.id.to_string(),
        exp: expiration,
    };

    // 4. Generate the actual JWT token string
    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 5. Return a 200 OK status with the token in JSON format
    Ok((StatusCode::OK, Json(AuthResponse { token })))
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
        .route("/auth/register", post(register_user))
        .route("/auth/login", post(login_user)) // <-- New route
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 Server running on http://localhost:3000");
    
    axum::serve(listener, app).await.unwrap();
}