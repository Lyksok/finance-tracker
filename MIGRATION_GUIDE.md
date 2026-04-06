# OAuth Account Migration Tool

The `migrate_account` CLI tool allows administrators to manually bind external OAuth accounts (like GitHub or Google) to an existing local user profile within the Finance Tracker app. This ensures that legacy users don't lose their data when transitioning to the new OAuth authentication system.

## When to use this tool
You should use this tool when a user already has an existing account (e.g., they registered via username/password) and they want to start logging in with GitHub or Google *without* creating a duplicate account and losing their existing database records.

## Usage

You must run the tool from the root of the `finance_tracker` project. The application server does not need to be shut down (SQLite handles the concurrent connections), but the `finance.db` file must exist in the directory.

```bash
cargo run --bin migrate_account -- --user-id <ID> --provider <PROVIDER> --provider-id <OAUTH_ID>
```

### Arguments:

*   `--user-id` (`-u`): The internal integer ID of the user in your database (e.g., `1`). This is the primary key mapping to existing records. 
*   `--provider` (`-p`): The name of the OAuth provider. Currently supported values: `github` or `google`.
*   `--provider-id`: The unique identification string provided by the OAuth service. For GitHub, this is an integer-based string (e.g., `94795120`). For Google, this is a long numeric string.

### Step-by-Step Example

**Scenario:** The administrator wants to link their GitHub account to their local `admin` profile (which has a User ID of `1`).

1. **Find the User ID:** If you don't know the exact `user_id`, you can query it via the sqlite CLI:
   ```bash
   sqlite3 finance.db "SELECT id, username FROM users;"
   ```
2. **Find the Provider ID:** When the user attempts to log in via GitHub but is unbound, the application log will print a warning containing their exact Provider ID:
   ```log
   WARN Unbound OAuth user tried to login: github - 94795120
   ```
3. **Execute the Bind:** Run the tool with the gathered parameters:
   ```bash
   cargo run --bin migrate_account -- --user-id 1 --provider github --provider-id 94795120
   ```
4. **Result:** The system will output: `Successfully bound user ID 1 to github ID '94795120'.` The user can immediately click the "Login with GitHub" button and they will be seamlessly logged into their existing profile.

## Troubleshooting

*   **Error: User with ID X not found:** Ensure you are passing the correct numeric database ID for the user, not their string username.
*   **Failed to connect to database:** Guarantee you are running the command exclusively from the root dir where `finance.db` is located.
