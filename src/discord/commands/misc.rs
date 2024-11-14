use poise::serenity_prelude as serenity;
use poise::serenity_prelude::colours::branding::YELLOW;
use poise::serenity_prelude::colours::roles::{BLUE, GREEN, RED};
use crate::discord::bot::{CommandError, Context};

#[poise::command(slash_command, prefix_command)]
pub async fn ping(
    ctx: Context<'_>,
) -> Result<(), CommandError> {
    let ping = ctx.ping().await.as_millis();
    let color = if ping == 0 {
        BLUE
    } else if ping < 100 {
        GREEN
    } else if ping < 250 {
        YELLOW
    } else {
        RED
    };

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::default()
                .title("Pong!")
                .description(format!("Gateway latency: {latency}ms", latency = ping))
                .color(color),
        ),
    ).await?;
    Ok(())
}
