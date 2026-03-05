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
3. Set up your environment file. Copy the `.env.example` file and configure your port:
   ```bash
   cp .env.example .env
   ```
4. Run the application:
   ```bash
   cargo run
   ```
5. Open your browser and go to `http://localhost:<PORT>/login`, replacing `<PORT>` with the value in your `.env` (defaults to 8000).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
