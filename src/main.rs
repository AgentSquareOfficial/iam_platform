// Declare our new modules so the Rust compiler knows they exist
mod handlers;
mod middleware;
mod models;

use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;

// Import the functions we need from our new modules
use handlers::auth::{get_me, login_user, register_user};
use middleware::auth::auth_middleware;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to Postgres.");

    println!("✅ Successfully connected to the database!");

    // Protected routes
    let api_routes = Router::new()
        .route("/users/me", get(get_me))
        .route_layer(axum::middleware::from_fn(auth_middleware));

    // Public routes
    let public_routes = Router::new()
        .route("/", get(|| async { "Hello, IAM Platform! Connected to DB." }))
        .route("/auth/register", post(register_user))
        .route("/auth/login", post(login_user));

    let app = Router::new()
        .merge(public_routes)
        .merge(api_routes)
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 Server running on http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}