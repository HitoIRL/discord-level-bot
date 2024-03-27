use chrono::Utc;
use redis::AsyncCommands;
use ::serenity::all::{ChannelId, MessageFlags};
use serenity::all::{Color, CreateEmbed, CreateEmbedFooter, CreateMessage, Mentionable, Message, Timestamp};
use poise::serenity_prelude as serenity;

use crate::{Data, LevelsConfig, Result, User};

pub async fn on_message(ctx: &serenity::Context, data: &Data, msg: &Message) -> Result {
    let mut redis = data.redis.get_multiplexed_async_connection().await?;
    let postgres = data.postgres.clone();

    // check if user is on cooldown
    let user_id = msg.author.id.get();
    let cooldown: Option<i64> = redis.get(user_id).await?;
    if let Some(cooldown) = cooldown {
        if cooldown > Utc::now().timestamp() {
            println!("User {user_id} is on cooldown");
            return Ok(());
        }
    }

    // set user on cooldown
    let cooldown = (Utc::now() + data.config.levels.cooldown).timestamp();
    let _: () = redis.set(user_id, cooldown).await?;
    println!("User {user_id} is now on cooldown for {} seconds", data.config.levels.cooldown.num_seconds());

    // fetch user details from postgres
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id as i64)
        .fetch_optional(&postgres)
        .await?;

    // if user does not exist, create a new user
    let mut user = match user {
        Some(user) => user,
        None => sqlx::query_as!(User, "INSERT INTO users (id) VALUES ($1) RETURNING *", user_id as i64)
                    .fetch_one(&postgres)
                    .await?
    };

    // update user xp and level
    user.xp += data.config.levels.message_xp;
    println!("User {user_id} has gained {} xp!", data.config.levels.message_xp);

    let mut required_xp = xp_required_for_level(&data.config.levels, user.level + 1);
    while user.xp >= required_xp {
        user.level += 1;
        user.xp -= xp_required_for_level(&data.config.levels, user.level);
        println!("User {user_id} has leveled up to level {}. Now requires {} xp to next level", user.level, xp_required_for_level(&data.config.levels, user.level + 1) - user.xp);
        
        required_xp = xp_required_for_level(&data.config.levels, user.level + 1);

        let embed = CreateEmbed::default()
            .thumbnail(msg.author.avatar_url().unwrap_or_default())
            .title("Nowy poziom")
            .description("Udało ci się zdobyć nowy poziom! Dziękujemy za twoją aktywność ❤️")
            .field("Poziom", format!("{}", user.level), true)
            .field("XP", format!("{} / {required_xp}", user.xp), true)
            .color(Color::from(0xa7b1fd))
            .timestamp(Timestamp::now())
            .footer(CreateEmbedFooter::new("Homies - Wspieramy edukację domową"));
        let message = CreateMessage::new()
            .content(format!("{}", msg.author.mention()))
            .embed(embed)
            .flags(MessageFlags::SUPPRESS_NOTIFICATIONS);

        let channel_id = data.config.levels.channel;
        let channel = ChannelId::new(channel_id as u64);
        channel.send_message(&ctx, message).await?;
    }
    println!("For next level up, user {user_id} requires {} xp", xp_required_for_level(&data.config.levels, user.level + 1) - user.xp);

    // update user details in postgres
    sqlx::query!("UPDATE users SET xp = $1, level = $2 WHERE id = $3", user.xp, user.level, user_id as i64)
        .execute(&postgres)
        .await?;

    Ok(())
}

pub fn xp_required_for_level(config: &LevelsConfig, level: i32) -> i32 {
    (config.base_xp as f32 * (config.xp_factor.powi(level as i32))).round() as i32
}
