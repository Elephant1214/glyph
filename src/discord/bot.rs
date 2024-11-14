use crate::discord::commands;
use crate::{ChannelCommand, GlyphState};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::ActivityData;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::oneshot::Sender;

pub struct BotState {
    pub(crate) global_state: Arc<GlyphState>,
    tx: Sender<ChannelCommand>,
}

pub type CommandError = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, BotState, CommandError>;

pub(crate) async fn start_bot(state: Arc<GlyphState>, tx: Sender<ChannelCommand>) -> serenity::Result<serenity::Client> {
    let token = std::env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::all();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![commands::misc::ping(), commands::user::create_user()],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("glyph!".into()),
                ..Default::default()
            },
            owners: HashSet::from([serenity::UserId::from(363474138549059604)]),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(BotState {
                    global_state: state,
                    tx,
                })
            })
        })
        .build();

    serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .activity(ActivityData::watching("Glyph servers"))
        .await
}
