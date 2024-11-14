use poise::serenity_prelude as serenity;
use poise::serenity_prelude::colours::roles::GREEN;
use crate::discord::bot::{CommandError, Context};
use crate::user;

#[poise::command(slash_command, prefix_command, owners_only = true)]
pub async fn create_user(
    ctx: Context<'_>,
    user: serenity::User,
    display_name: Option<String>,
) -> Result<(), CommandError> {
    let display_name = if display_name.is_some() {
        display_name.unwrap()
    } else {
        user.name
    };
    match user::create_user(&ctx.data().global_state.mongo, user.id.into(), display_name.to_string()).await {
        Ok(val) => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::default()
                        .title("User Created")
                        .description(format!("Account ID: {acc_id}\nDisplay name: {name}\nDiscord ID: {id}", acc_id = val.account_id, name = val.display_name, id = val.discord_id))
                        .color(GREEN),
                ),
            ).await?;
            Ok(())
        },
        Err(e) => {
            ctx.reply(format!("Failed to create user: {}", e)).await?;
            Ok(())
        }
    }
}
