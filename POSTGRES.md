# PostgreSQL Setup Guide for Boticelli

This guide walks you through setting up PostgreSQL from scratch for use with Boticelli. Whether you're new to PostgreSQL or just need a refresher, this will get you up and running.

## Table of Contents

1. [Installation](#installation)
2. [Initial Setup](#initial-setup)
3. [Creating Database and User](#creating-database-and-user)
4. [Configuring Boticelli](#configuring-boticelli)
5. [Running Migrations](#running-migrations)
6. [Testing the Connection](#testing-the-connection)
7. [Troubleshooting](#troubleshooting)
8. [Common Commands Reference](#common-commands-reference)

## Installation

### Arch Linux / Manjaro

```bash
# Install PostgreSQL
sudo pacman -S postgresql

# Initialize the database cluster
sudo -u postgres initdb --locale=C.UTF-8 --encoding=UTF8 -D '/var/lib/postgres/data'

# Start PostgreSQL service
sudo systemctl start postgresql

# Enable PostgreSQL to start on boot (optional)
sudo systemctl enable postgresql

# Verify it's running
systemctl status postgresql
```

### Ubuntu / Debian

```bash
# Install PostgreSQL
sudo apt update
sudo apt install postgresql postgresql-contrib

# PostgreSQL starts automatically on these systems
sudo systemctl status postgresql
```

### macOS

```bash
# Using Homebrew
brew install postgresql@14

# Start PostgreSQL
brew services start postgresql@14

# Check status
brew services list | grep postgresql
```

### Windows

1. Download the installer from [postgresql.org](https://www.postgresql.org/download/windows/)
2. Run the installer and follow the prompts
3. Remember the password you set for the `postgres` superuser
4. The service starts automatically

## Initial Setup

After installation, PostgreSQL creates a superuser account called `postgres`. You'll use this account to create your project-specific database and user.

### Step 1: Access PostgreSQL as the postgres User

**Linux/macOS:**
```bash
sudo -u postgres psql
```

**Windows:**
```bash
# Open Command Prompt or PowerShell
psql -U postgres
```

You should see a prompt that looks like:
```
postgres=#
```

This means you're connected to PostgreSQL!

## Creating Database and User

Now we'll create a dedicated user and database for Boticelli. Run these commands in the `psql` prompt:

### Step 2: Create the Boticelli User (Role)

```sql
CREATE USER boticelli WITH PASSWORD 'renaissance';
```

**Note:** Replace `'renaissance'` with a secure password of your choice. Remember this password - you'll need it in your `.env` file.

You should see:
```
CREATE ROLE
```

### Step 3: Create the Boticelli Database

```sql
CREATE DATABASE boticelli OWNER boticelli;
```

You should see:
```
CREATE DATABASE
```

### Step 4: Grant Privileges

```sql
GRANT ALL PRIVILEGES ON DATABASE boticelli TO boticelli;
```

You should see:
```
GRANT
```

### Step 5: Exit psql

```sql
\q
```

This returns you to your regular terminal prompt.

## Configuring Boticelli

Now configure Boticelli to use your new database by editing the `.env` file:

### Step 6: Create or Edit `.env`

```bash
# If .env doesn't exist, copy from the example
cp .env.example .env
```

### Step 7: Add Database Credentials

Edit `.env` and add these lines (or uncomment if they exist):

```env
DATABASE_USER=boticelli
DATABASE_PASSWORD=renaissance
DATABASE_NAME=boticelli
DATABASE_HOST=localhost
DATABASE_PORT=5432
```

**Important:** Use the password you chose in Step 2.

## Running Migrations

Migrations create the database tables that Boticelli needs to store execution history.

### Step 8: Install Diesel CLI (One-Time Setup)

```bash
cargo install diesel_cli --no-default-features --features postgres
```

This may take a few minutes as it compiles.

### Step 9: Run Migrations

```bash
# From the boticelli project directory
diesel migration run
```

You should see output like:
```
Running migration 2024-01-10-000000_create_model_responses
Running migration 2025-01-10-000000_create_narrative_executions
```

## Testing the Connection

### Step 10: Test with psql

Verify you can connect with the boticelli user:

```bash
psql -U boticelli -h localhost -d boticelli
```

Enter the password when prompted. If successful, you'll see:
```
boticelli=#
```

Type `\dt` to see the tables created by migrations:
```sql
\dt
```

You should see tables like:
```
              List of relations
 Schema |         Name          | Type  |   Owner
--------+-----------------------+-------+-----------
 public | __diesel_schema_migrations | table | boticelli
 public | act_executions        | table | boticelli
 public | act_inputs            | table | boticelli
 public | model_responses       | table | boticelli
 public | narrative_executions  | table | boticelli
```

Exit psql:
```sql
\q
```

### Step 11: Test with Boticelli

Try running a narrative with the `--save` flag:

```bash
cargo run --release -- run -n narrations/mint.toml --save --verbose
```

If everything is set up correctly, you should see:
```
💾 Saving to database...
✓ Saved as execution ID: 1
```

## Troubleshooting

### Error: "role 'boticelli' does not exist"

**Problem:** The user was not created successfully.

**Solution:**
```bash
# Connect as postgres superuser
sudo -u postgres psql

# Create the user
CREATE USER boticelli WITH PASSWORD 'your_password';

# Exit
\q
```

### Error: "database 'boticelli' does not exist"

**Problem:** The database was not created successfully.

**Solution:**
```bash
# Connect as postgres superuser
sudo -u postgres psql

# Create the database
CREATE DATABASE boticelli OWNER boticelli;

# Exit
\q
```

### Error: "connection refused" or "could not connect to server"

**Problem:** PostgreSQL service is not running.

**Solution:**

**Linux:**
```bash
# Check status
systemctl status postgresql

# Start if not running
sudo systemctl start postgresql
```

**macOS:**
```bash
brew services start postgresql@14
```

**Windows:**
Open Services app and start "postgresql" service.

### Error: "permission denied for database"

**Problem:** The boticelli user doesn't have the right privileges.

**Solution:**
```bash
# Connect as postgres superuser
sudo -u postgres psql

# Grant privileges
GRANT ALL PRIVILEGES ON DATABASE boticelli TO boticelli;

# Also grant schema privileges
\c boticelli
GRANT ALL ON SCHEMA public TO boticelli;

# Exit
\q
```

### Error: "diesel migration run" fails

**Problem:** Diesel CLI might not be installed or DATABASE_URL is wrong.

**Solution:**
```bash
# Reinstall diesel CLI
cargo install diesel_cli --no-default-features --features postgres --force

# Check your .env file has correct credentials
cat .env | grep DATABASE

# Try running migrations again
diesel migration run
```

### Error: "relation '__diesel_schema_migrations' does not exist"

**Problem:** Diesel setup hasn't been run.

**Solution:**
```bash
# Set up diesel (creates migrations table)
diesel setup

# Run migrations
diesel migration run
```

### Cannot connect as postgres user on Linux

**Problem:** Permission issues with the postgres system user.

**Solution:**
```bash
# Switch to postgres user, then run psql
sudo su - postgres
psql

# When done, exit psql and return to your user
\q
exit
```

## Common Commands Reference

### PostgreSQL Service Management

**Linux (systemd):**
```bash
sudo systemctl start postgresql    # Start service
sudo systemctl stop postgresql     # Stop service
sudo systemctl restart postgresql  # Restart service
sudo systemctl status postgresql   # Check status
sudo systemctl enable postgresql   # Enable on boot
```

**macOS (Homebrew):**
```bash
brew services start postgresql@14
brew services stop postgresql@14
brew services restart postgresql@14
```

### Connecting to PostgreSQL

```bash
# Connect as postgres superuser
sudo -u postgres psql

# Connect as specific user to specific database
psql -U boticelli -h localhost -d boticelli

# Connect using connection URL
psql postgres://boticelli:password@localhost:5432/boticelli
```

### Common psql Commands

Once inside `psql`:

```sql
\l              -- List all databases
\c dbname       -- Connect to database 'dbname'
\dt             -- List all tables in current database
\d tablename    -- Describe table structure
\du             -- List all users (roles)
\q              -- Quit psql
\?              -- Help on psql commands
\h              -- Help on SQL commands

-- SQL commands end with semicolon
SELECT version();           -- Show PostgreSQL version
SELECT current_user;        -- Show current user
SELECT current_database();  -- Show current database
```

### User and Database Management

```sql
-- Create user
CREATE USER username WITH PASSWORD 'password';

-- Create database
CREATE DATABASE dbname OWNER username;

-- Grant privileges
GRANT ALL PRIVILEGES ON DATABASE dbname TO username;

-- Change user password
ALTER USER username WITH PASSWORD 'newpassword';

-- Drop user (careful!)
DROP USER username;

-- Drop database (careful!)
DROP DATABASE dbname;
```

### Diesel CLI Commands

```bash
# Set up diesel (first time only)
diesel setup

# Create a new migration
diesel migration generate migration_name

# Run pending migrations
diesel migration run

# Revert last migration
diesel migration revert

# Redo last migration (revert then run)
diesel migration redo

# Show migration status
diesel migration list

# Completely reset database (WARNING: destroys all data)
diesel database reset
```

### Checking Boticelli Tables

```sql
-- Connect to boticelli database
psql -U boticelli -h localhost -d boticelli

-- List all tables
\dt

-- Check narrative executions
SELECT id, narrative_name, status, started_at FROM narrative_executions;

-- Check acts for a specific execution
SELECT id, act_name, sequence_number, model FROM act_executions WHERE execution_id = 1;

-- Check inputs for a specific act
SELECT id, input_type, input_order FROM act_inputs WHERE act_execution_id = 1;

-- Count total executions
SELECT COUNT(*) FROM narrative_executions;
```

## Next Steps

Once PostgreSQL is set up and working:

1. ✅ PostgreSQL is installed and running
2. ✅ `boticelli` user and database are created
3. ✅ `.env` file is configured with credentials
4. ✅ Migrations have been run successfully
5. ✅ Connection test succeeded

You're ready to use Boticelli with database persistence! Try running:

```bash
# Run narrative and save to database
cargo run --release -- run -n narrations/mint.toml --save --verbose

# List saved executions
cargo run --release -- list

# Show execution details
cargo run --release -- show 1
```

## Additional Resources

- [PostgreSQL Official Documentation](https://www.postgresql.org/docs/)
- [PostgreSQL Tutorial](https://www.postgresqltutorial.com/)
- [Diesel ORM Documentation](https://diesel.rs/)
- [Arch Wiki: PostgreSQL](https://wiki.archlinux.org/title/PostgreSQL)

## Getting Help

If you encounter issues not covered in this guide:

1. Check the [main README.md](README.md) troubleshooting section
2. Look at PostgreSQL logs:
   - Linux: `sudo journalctl -u postgresql`
   - macOS: `tail -f /usr/local/var/log/postgres.log`
   - Windows: Check Event Viewer
3. Open an issue on the [Boticelli GitHub repository](https://github.com/crumplecup/boticelli/issues)
