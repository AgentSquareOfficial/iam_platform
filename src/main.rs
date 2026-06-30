use axum::{routing::get, Router};
use sqlx::postgres::PgPoolOptions;
use dotenvy::dotenv;
use std::env;

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

    // Inject the database pool into our Axum application state
    let app = Router::new()
        .route("/", get(|| async { "Hello, IAM Platform! Connected to DB." }))
        .with_state(pool); // allows our route handlers to access the database

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 Server running on http://localhost:3000");
    
    axum::serve(listener, app).await.unwrap();
}