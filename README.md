# Solo Finance Watcher

A fast, private, and simple personal finance tracking application written in Rust.

## Features

- **Dashboard**: Get a quick overview of your income, expenses, and net balance by month.
- **Categorization**: Create custom categories to group your transactions.
- **Transaction Tracking**: Add, edit, and categorize your transactions easily.
- **Privacy First**: All data is stored in a local SQLite database (`finance.db`).

## Tech Stack

- **Backend**: [Rust](https://www.rust-lang.org/) + [Axum](https://github.com/tokio-rs/axum)
- **Database**: [SQLite](https://sqlite.org/index.html) + [sqlx](https://github.com/launchbadge/sqlx)
- **Templating**: [Askama](https://github.com/djc/askama) 
- **Styling**: Vanilla CSS (Modern Dark Theme)

## Getting Started

1. Ensure you have [Rust and Cargo installed](https://rustup.rs/).
2. Clone the repository and navigate to the project folder:
   ```bash
   git clone git@github.com:Lyksok/finance-tracker.git
   cd finance-tracker
   ```
3. Set up your environment file. Copy the `.env.example` file to `.env`:
   ```bash
   cp .env.example .env
   ```
4. Define your environment variables inside `.env`:
   - `PORT`: (Optional) The local port the Axum app binds to (default: `8000`).
   - `DOMAIN`: (Optional) The hostname Caddy should secure via TLS (default: `localhost`).
   - `EXTERNAL_PORT`: (Optional) The public port Caddy binds to (default: `3000`).
5. Run the application:
   ```bash
   cargo run
   ```

## Deployment & Using HTTPS

This project includes a `Caddyfile` for automatically providing secure HTTPS connections locally or on a production server. The proxy is entirely configurable via the `.env` file.

1. Ensure [Caddy](https://caddyserver.com/docs/install) is installed.
2. Run your Axum backend in a separate terminal or background it:
   ```bash
   cargo run --release
   ```
3. Start the Caddy reverse proxy. It will automatically read the `.env` file and provision certificates for your custom `$DOMAIN`:
   ```bash
   sudo caddy start --config Caddyfile --envfile .env
   ```
4. Open your browser and navigate to **`https://<DOMAIN>:<EXTERNAL_PORT>`** (e.g., `https://localhost:3000`). Do **NOT** access the internal `PORT` directly as secure session cookies will not work over unencrypted HTTP.

## Running in the Background on Linux (Using `screen`)

If you are deploying this on a Linux server and want the application to stay running after you close your SSH session, you can use `screen`:

1. Install screen if you haven't already:
   ```bash
   sudo apt update && sudo apt install screen
   ```
2. Start a new screen session for the Axum backend:
   ```bash
   screen -S finance-backend
   ```
3. Navigate to the project directory and run the app in release mode:
   ```bash
   cargo run --release
   ```
4. **Detach** from the screen session by pressing `Ctrl+A`, then `D`. The app is now running in the background!
5. Start Caddy in the background using its built-in daemon mode, pointing it to your `.env` file:
   ```bash
   sudo caddy start --config Caddyfile --envfile .env
   ```

To reattach to the backend later and view the logs, type `screen -r finance-backend`.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
