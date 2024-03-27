use poise::command;
use poise::CreateReply;
use ::serenity::all::CreateEmbed;
use ::serenity::all::CreateEmbedFooter;
use ::serenity::all::Timestamp;
use ::serenity::all::UserId;

use crate::leveling::xp_required_for_level;
use crate::{Context, Result, User};

#[command(slash_command, rename = "ranking")]
pub async fn leaderboard(
    ctx: Context<'_>,
) -> Result {
    // TODO: full blown leaderboard with pagination

    // get top 10 users from database sorted by level
    let users = sqlx::query_as!(User, "SELECT * FROM users ORDER BY level DESC LIMIT 10")
        .fetch_all(&ctx.data().postgres)
        .await?;

    // create codeblock with top 10 users
    // [place] name level (xp / required xp)
    let mut description = String::new();
    for (i, user) in users.iter().enumerate() {
        let xp_required = xp_required_for_level(&ctx.data().config.levels, user.level + 1);
        description.push_str(&format!(
            "{place}. **{name}**, Poziom: {level} ({xp}xp / {xp_required}xp)\n",
            place = i + 1,
            name = UserId::new(user.id as u64).to_user(&ctx).await?.global_name.unwrap(),
            level = user.level,
            xp = user.xp,
            xp_required = xp_required
        ));
    }

    // send user level details in embed
    let embed = CreateEmbed::default()
        .title("Ranking użytkowników")
        .description(description)
        .color(0xa7b1fd)
        .timestamp(Timestamp::now())
        .footer(CreateEmbedFooter::new("Homies - Wspieramy edukację domową"));
    let reply = CreateReply::default()
        .embed(embed);

    ctx.send(reply).await?;

    Ok(())
}
