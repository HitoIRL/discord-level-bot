use poise::command;
use poise::serenity_prelude as serenity;
use poise::CreateReply;
use ::serenity::all::CreateEmbed;
use ::serenity::all::CreateEmbedFooter;
use ::serenity::all::Timestamp;

use crate::leveling::xp_required_for_level;
use crate::{Context, Result, User};

#[command(slash_command, rename = "poziom")]
pub async fn level(
    ctx: Context<'_>,
    #[description = "Użytkownik, którego poziom chcesz sprawdzić"]
    user: Option<serenity::User>,
) -> Result {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());

    // check for user details in database
    let res = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user.id.get() as i64)
        .fetch_optional(&ctx.data().postgres)
        .await?;

    // if user does not exist, return message
    let user_details = match res {
        Some(user) => user,
        None => {
            let reply = CreateReply::default()
                .content("Nie znaleziono użytkownika w bazie danych")
                .ephemeral(true);
            ctx.send(reply).await?;
            return Ok(());
        }
    };

    // send user level details in embed
    let xp_required = xp_required_for_level(&ctx.data().config.levels, user_details.level + 1);
    let embed = CreateEmbed::default()
        .thumbnail(user.avatar_url().unwrap_or_default())
        .title("Poziom użytkownika")
        .field("Poziom", format!("{}", user_details.level), true)
        .field("XP", format!("{} / {xp_required}", user_details.xp), true)
        .color(0xa7b1fd)
        .timestamp(Timestamp::now())
        .footer(CreateEmbedFooter::new("Homies - Wspieramy edukację domową"));
    let reply = CreateReply::default()
        .embed(embed);

    ctx.send(reply).await?;

    Ok(())
}