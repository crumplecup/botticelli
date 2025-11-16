# Discord Bot Setup Guide

This guide explains how to create a Discord bot, get your bot token, and configure it for use with Boticelli.

## Prerequisites

- A Discord account
- Administrative access to a Discord server (or create your own test server)

## Step 1: Create a Discord Application

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Click **"New Application"** in the top-right corner
3. Enter a name for your application (e.g., "Boticelli Bot")
4. Read and accept the Discord Developer Terms of Service
5. Click **"Create"**

You should now see your application's dashboard.

## Step 2: Create a Bot User

1. In the left sidebar, click **"Bot"**
2. Click **"Add Bot"** (or **"Reset Token"** if the bot already exists)
3. Confirm by clicking **"Yes, do it!"**

Your bot is now created!

## Step 3: Get Your Bot Token

⚠️ **Important**: Your bot token is like a password. Never share it publicly or commit it to Git!

1. On the Bot page, find the **"TOKEN"** section
2. Click **"Reset Token"** (you may need to enter your 2FA code)
3. Click **"Copy"** to copy your token to the clipboard
4. Save this token immediately - you'll need it in Step 6

## Step 4: Configure Bot Permissions and Intents

### Required Privileged Gateway Intents

Boticelli requires specific gateway intents to function properly:

1. On the Bot page, scroll down to **"Privileged Gateway Intents"**
2. Enable the following intents:
   - ✅ **PRESENCE INTENT** (optional, for member status)
   - ✅ **SERVER MEMBERS INTENT** (required for member events)
   - ✅ **MESSAGE CONTENT INTENT** (required for message content)

### Bot Permissions

Boticelli needs certain permissions to read Discord data:

1. In the left sidebar, click **"OAuth2"** → **"URL Generator"**
2. Under **"SCOPES"**, select:
   - ✅ `bot`
3. Under **"BOT PERMISSIONS"**, select:
   - ✅ **View Channels** (Read Messages/View Channels)
   - ✅ **Read Message History**
   - ✅ **Send Messages** (for future narrative posting)
   - ✅ **Embed Links** (for rich message formatting)
   - ✅ **Attach Files** (for media sharing)

4. Scroll down and copy the **Generated URL** at the bottom

## Step 5: Invite the Bot to Your Server

1. Paste the URL you copied into your web browser
2. Select the Discord server you want to add the bot to
   - You must have **"Manage Server"** permission on the server
   - If you don't have a server, create one: Discord → "+" → "Create My Own" → "For me and my friends"
3. Click **"Authorize"**
4. Complete the CAPTCHA if prompted

Your bot is now in your server! It will appear offline until you start it.

## Step 6: Configure Your Environment

### Option 1: Using a .env File (Recommended)

1. In your Boticelli project directory, create a `.env` file:

```bash
# Create or edit .env file
nano .env
```

2. Add your Discord token:

```env
# Discord Bot Configuration
DISCORD_TOKEN=your_bot_token_here

# Database URL (if not already set)
DATABASE_URL=postgres://username:password@localhost/boticelli
```

3. Save the file (Ctrl+O, Enter, Ctrl+X in nano)

⚠️ **Important**: Make sure `.env` is in your `.gitignore` file:

```bash
# Check if .env is ignored
grep -q "^\.env$" .gitignore || echo ".env" >> .gitignore
```

### Option 2: Using Environment Variables

Alternatively, export the token directly in your shell:

```bash
export DISCORD_TOKEN="your_bot_token_here"
```

Note: This only lasts for your current terminal session.

### Option 3: Using the CLI Argument

You can also pass the token via command line (not recommended for security):

```bash
boticelli discord start --token "your_bot_token_here"
```

## Step 7: Run the Database Migrations

Before starting the bot, ensure Discord tables exist in your database:

```bash
# Run Diesel migrations
diesel migration run
```

You should see output like:

```
Running migration 2025-11-15-233432-0000_create_discord_guilds
Running migration 2025-11-15-233458-0000_create_discord_users
Running migration 2025-11-15-233523-0000_create_discord_channels
Running migration 2025-11-15-233550-0000_create_discord_guild_members
Running migration 2025-11-15-233608-0000_create_discord_roles
Running migration 2025-11-15-233628-0000_create_discord_member_roles
```

## Step 8: Start Your Bot

Build and run the bot with the Discord feature enabled:

```bash
# Build with Discord feature
cargo build --release --features discord

# Start the bot
cargo run --release --features discord -- discord start
```

Or using the binary directly:

```bash
# Using .env or environment variable
./target/release/boticelli discord start

# Using CLI argument
./target/release/boticelli discord start --token "your_bot_token_here"
```

You should see output like:

```
🤖 Starting Boticelli Discord bot...
✓ Bot initialized successfully
🚀 Connecting to Discord...
   (Press Ctrl+C to stop)

Bot connected to Discord
```

## Step 9: Verify It's Working

### Check the Bot Status

1. In Discord, your bot should now show as **Online** (green dot)
2. Check the console output for connection messages

### Verify Database Storage

Check that Discord data is being stored:

```bash
# Connect to your database
psql $DATABASE_URL

# Check stored guilds
SELECT id, name, member_count FROM discord_guilds;

# Check stored channels
SELECT id, name, channel_type FROM discord_channels LIMIT 10;

# Check stored users
SELECT id, username, bot FROM discord_users LIMIT 10;
```

### Test Event Capture

Try these actions in your Discord server and verify they're logged:

1. **Create a channel** - Check console for "Channel created" log
2. **Create a role** - Check console for "Role created" log
3. **Invite a friend** or use an alt account to join - Check for "Member joined guild" log

## Troubleshooting

### Bot Shows as Offline

- Verify your token is correct (try regenerating it)
- Check that the bot process is running
- Look for error messages in the console

### "Invalid Token" Error

- Make sure you copied the entire token
- Regenerate the token in the Discord Developer Portal
- Check for extra spaces or quotes in your .env file

### "Missing Privileged Intent" Error

- Go back to the Bot page in Discord Developer Portal
- Enable **SERVER MEMBERS INTENT** and **MESSAGE CONTENT INTENT**
- Restart your bot

### Database Connection Errors

- Verify `DATABASE_URL` is set correctly
- Check that PostgreSQL is running: `systemctl status postgresql`
- Ensure migrations have been run: `diesel migration run`

### Permission Errors

- Check that the bot has the required permissions in your server
- Right-click the bot in Discord → "Edit Permissions"
- Ensure "View Channels" and "Read Messages" are enabled

## Security Best Practices

### Protect Your Token

1. ✅ **Never** commit your token to Git
2. ✅ Use `.env` files for local development
3. ✅ Use environment variables for production
4. ✅ Regenerate your token if it's ever leaked
5. ✅ Keep `.env` in your `.gitignore`

### Token Leaked?

If your token is accidentally exposed:

1. Go to Discord Developer Portal → Your Application → Bot
2. Click **"Reset Token"**
3. Update your `.env` file with the new token
4. Restart your bot

### Rotate Tokens Regularly

For production bots, consider rotating tokens periodically:

1. Generate a new token in the Developer Portal
2. Update your deployment configuration
3. Invalidate the old token

## Next Steps

Now that your bot is running:

1. **Monitor the logs** - Use `RUST_LOG=debug` for verbose output:
   ```bash
   RUST_LOG=debug cargo run --features discord -- discord start
   ```

2. **Explore the database** - Query Discord data:
   ```bash
   psql $DATABASE_URL -c "SELECT name, member_count FROM discord_guilds;"
   ```

3. **Read the schema** - See [DISCORD_SCHEMA.md](DISCORD_SCHEMA.md) for full database structure

4. **Review the code** - Check [SOCIAL_MEDIA.md](SOCIAL_MEDIA.md) for implementation details

## Reference Links

- [Discord Developer Portal](https://discord.com/developers/applications)
- [Discord Developer Documentation](https://discord.com/developers/docs)
- [Serenity Discord Library](https://docs.rs/serenity/)
- [Discord Bot Best Practices](https://discord.com/developers/docs/topics/gateway#privileged-intents)

## Support

For issues specific to Boticelli's Discord integration, check:

- Console error messages (enable with `RUST_LOG=debug`)
- Database logs in PostgreSQL
- Discord API status: <https://discordstatus.com/>

For general Discord bot questions:

- [Discord Developers Discord Server](https://discord.gg/discord-developers)
- [Serenity Discord Server](https://discord.gg/serenity-rs)
