# botticelli_social

Social platform integrations for the Botticelli ecosystem.

## Overview

Integration with social platforms, currently supporting Discord. Store and query Discord data (guilds, channels, members, messages) in PostgreSQL.

## Features

### Discord Support

```toml
[dependencies]
botticelli_social = { version = "0.2", features = ["discord"] }
```

- **Guild management**: Store server information
- **Channel tracking**: Text, voice, announcement channels
- **Member data**: Users, roles, permissions
- **Message history**: Chat messages with metadata
- **Repository pattern**: Type-safe database operations

## Usage

### Discord Bot Setup

```rust
use botticelli_social::DiscordDriver;
use serenity::prelude::*;

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_BOT_TOKEN")?;
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await?;
        
    client.start().await?;
}
```

### Storing Discord Data

```rust
use botticelli_social::discord::{
    DiscordRepository, NewGuild, NewChannel, NewMember
};

let repo = DiscordRepository::new(&mut conn)?;

// Store guild
let guild = NewGuild {
    guild_id: 123456789,
    name: "My Server".to_string(),
    icon: None,
    owner_id: 987654321,
};
repo.upsert_guild(&guild)?;

// Store channel
let channel = NewChannel {
    channel_id: 111222333,
    guild_id: 123456789,
    name: "general".to_string(),
    channel_type: "text".to_string(),
    position: 0,
};
repo.upsert_channel(&channel)?;

// Store member
let member = NewMember {
    guild_id: 123456789,
    user_id: 555666777,
    username: "alice".to_string(),
    discriminator: "0001".to_string(),
    nickname: Some("Alice".to_string()),
    joined_at: Utc::now(),
};
repo.upsert_member(&member)?;
```

### Querying Data

```rust
// Get all guilds
let guilds = repo.list_guilds()?;

// Get channels for a guild
let channels = repo.list_channels_for_guild(123456789)?;

// Get members for a guild
let members = repo.list_members_for_guild(123456789)?;

// Get messages for a channel
let messages = repo.list_messages_for_channel(111222333, 100)?;
```

### JSON Conversions

Convert between Serenity types and database models:

```rust
use botticelli_social::discord::conversions::{
    guild_to_new_guild, channel_to_new_channel, user_to_new_member
};
use serenity::model::prelude::*;

// Convert Serenity Guild to database model
let serenity_guild: Guild = /* ... */;
let db_guild = guild_to_new_guild(&serenity_guild);
repo.upsert_guild(&db_guild)?;

// Convert from JSON
let guild_json = r#"{"id": "123", "name": "My Server"}"#;
let new_guild = serde_json::from_str::<NewGuild>(guild_json)?;
```

## Database Schema

### Discord Tables

- `guilds` - Discord servers
  - guild_id, name, icon, owner_id, created_at
- `channels` - Server channels
  - channel_id, guild_id, name, channel_type, position
- `members` - Server members
  - guild_id, user_id, username, discriminator, nickname, joined_at
- `roles` - Server roles
  - role_id, guild_id, name, color, position, permissions
- `messages` - Chat messages
  - message_id, channel_id, author_id, content, timestamp

## Event Handling

```rust
use serenity::async_trait;
use serenity::model::prelude::*;
use serenity::prelude::*;

struct Handler {
    repo: DiscordRepository,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        // Store message
        let new_message = message_to_new_message(&msg);
        self.repo.insert_message(&new_message).unwrap();
    }
    
    async fn guild_create(&self, _ctx: Context, guild: Guild) {
        // Store guild and all its data
        let new_guild = guild_to_new_guild(&guild);
        self.repo.upsert_guild(&new_guild).unwrap();
        
        for channel in guild.channels.values() {
            let new_channel = channel_to_new_channel(channel, guild.id);
            self.repo.upsert_channel(&new_channel).unwrap();
        }
    }
}
```

## Configuration

```toml
# botticelli.toml
[discord]
token = "your_bot_token_here"

[database]
# Required for Discord integration
host = "localhost"
user = "botticelli_user"
password = "secret"
database = "botticelli"
```

## Dependencies

- `serenity` - Discord bot library
- `botticelli_interface` - Trait definitions
- `botticelli_database` - Database operations
- `diesel` - ORM
- `chrono` - Timestamps
- `serde` / `serde_json` - JSON handling

## Version

Current version: 0.2.0
