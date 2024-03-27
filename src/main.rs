mod commands;
mod leveling;

use chrono::Duration;
use commands::{leaderboard::leaderboard, level::level};
use dotenvy_macro::dotenv;
use serde::Deserialize;
use serde_with::serde_as;
use poise::serenity_prelude as serenity;
use serenity::all::{FullEvent, Message};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::fs;

const DATABASE_URL: &str = dotenv!("DATABASE_URL");

struct Data {
    config: Config,
    redis: redis::Client,
    postgres: PgPool,
}

type Result = std::result::Result<(), Error>;
type Error = anyhow::Error;
#[allow(unused)]
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Clone, Deserialize)]
struct DiscordConfig {
    token: String,
}

#[serde_as]
#[derive(Clone, Deserialize)]
struct LevelsConfig {
    channel: i64,
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    cooldown: Duration,
    message_xp: i32,
    base_xp: i32,
    xp_factor: f32,
}

#[derive(Clone, Deserialize)]
struct DatabaseConfig {
    redis: String,
}

#[derive(Clone, Deserialize)]
struct Config {
    discord: DiscordConfig,
    levels: LevelsConfig,
    database: DatabaseConfig,
}

#[derive(Deserialize)]
struct User {
    #[allow(dead_code)]
    id: i64,
    xp: i32,
    level: i32,
}

async fn on_message(ctx: &serenity::Context, data: &Data, msg: &Message) -> Result {
    // avoid processing bot messages and dms
    if msg.author.bot || !msg.guild_id.is_some() {
        return Ok(());
    }

    leveling::on_message(ctx, data, msg).await?;

    Ok(())
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data
) -> Result {
    match event {
        FullEvent::Ready { data_about_bot, .. } => {
            println!("{} is connected!", data_about_bot.user.name);
        }
        FullEvent::Message { new_message } => on_message(ctx, data, new_message).await?,
        _ => {}
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result {
    // config
    let config_file = fs::read_to_string("Config.toml").await?;
    let config: Config = toml::from_str(&config_file)?;

    let token = config.discord.token.clone();

    // redis client
    let redis = redis::Client::open(config.database.redis.clone())?;

    // postgres client
    let postgres = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await?;

    // bot framework
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            commands: vec![
                level(),
                leaderboard(),
            ],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    config,
                    redis,
                    postgres,
                })
            })
        })
        .build();

    let intents = serenity::GatewayIntents::non_privileged();
    let mut client = serenity::Client::builder(token, intents)
        .framework(framework)
        .await?;

    client.start().await?;
    Ok(())
}
