use clap::Parser;
use sqlx::sqlite::SqlitePoolOptions;
use std::process::exit;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Internal User ID of the existing account
    #[arg(short, long)]
    user_id: i64,

    /// OAuth provider (e.g., 'google', 'github')
    #[arg(short, long)]
    provider: String,

    /// Given OAuth provider ID for the user (e.g., email or unique string ID)
    #[arg(long)]
    provider_id: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let database_url = "sqlite:finance.db";
    let pool = SqlitePoolOptions::new()
        .connect(database_url)
        .await
        .unwrap_or_else(|_| {
            eprintln!("Failed to connect to the database. Ensure 'finance.db' exists.");
            exit(1);
        });

    // Check if the user exists
    let user_res: Result<Option<(i64,)>, _> = sqlx::query_as("SELECT id FROM users WHERE id = ?")
        .bind(args.user_id)
        .fetch_optional(&pool)
        .await;

    match user_res {
        Ok(Some((uid,))) => {
            let insert_res = sqlx::query(
                "INSERT INTO oauth_accounts (user_id, provider, provider_id) VALUES (?, ?, ?)"
            )
            .bind(uid)
            .bind(&args.provider)
            .bind(&args.provider_id)
            .execute(&pool)
            .await;

            match insert_res {
                Ok(_) => {
                    println!(
                        "Successfully bound user ID {} to {} ID '{}'.",
                        uid, args.provider, args.provider_id
                    );
                }
                Err(e) => {
                    eprintln!("Failed to bind account: {}", e);
                    exit(1);
                }
            }
        }
        Ok(None) => {
            eprintln!("Error: User with ID {} not found.", args.user_id);
            exit(1);
        }
        Err(e) => {
            eprintln!("Database error while searching for user: {}", e);
            exit(1);
        }
    }
}
