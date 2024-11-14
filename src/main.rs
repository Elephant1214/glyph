mod auth_manager;
mod serializers;
mod mongo;
mod error;
mod user;
mod util;

mod athena {
    mod items;
}

mod discord {
    pub mod bot;

    pub mod commands {
        pub mod misc;
        pub mod user;
    }
}

mod epic {
    pub(crate) mod epic_error;
}

mod route {
    pub mod account {
        pub mod auth;
    }
    pub(crate) mod router;
}

use crate::auth_manager::OAuthManager;
use crate::mongo::GlyphMongo;
use std::sync::{Arc};
use log::{error, info};
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;
use uuid::Uuid;
use route::router;

pub struct GlyphState {
    mongo: GlyphMongo,
    auth_manager: OAuthManager,
}

pub enum ChannelCommand {
    Shutdown {
        message: String,
    },
}

#[tokio::main]
async fn main() {
    let deployment_id = Uuid::new_v4();
    let signing_key = match OAuthManager::gen_signing_key(deployment_id.clone()) {
        Ok(val) => val,
        Err(e) => {
            error!("Unable to generate JWT signing key: {}", e);
            return;
        }
    };

    let shared_state = Arc::new(GlyphState {
        mongo: GlyphMongo::new().await.unwrap(),
        auth_manager: OAuthManager::new(signing_key),
    });
    let (tx, mut rx) = oneshot::channel::<ChannelCommand>();
    let token = tokio_util::sync::CancellationToken::new();
    let cloned_token = token.clone();

    tracing_subscriber::fmt::init();
    let app = router::create_router(shared_state.clone());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:5746").await.unwrap();

    tokio::spawn(async move {
        tokio::select! {
            _ = async {
                discord::bot::start_bot(shared_state.clone(), tx).await.unwrap().start().await.unwrap();
            } => {},
            _ = cloned_token.cancelled() => {},
        }
    });

    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(rx))
            .await
            .unwrap();
        token.cancel();
    });
}

async fn shutdown_signal(mut rx: Receiver<ChannelCommand>) {
    let terminate = async {
        vss::shutdown_signal()
    };
    let shutdown = async {
        match rx.try_recv() {
            Ok(cmd) => match cmd {
                ChannelCommand::Shutdown { message } => {
                    info!("Shutting down: {:?}", message)
                }
            }
            Err(_) => error!("Receiver dropped"),
        }
    };

    tokio::select! {
        _ = terminate => {},
        _ = shutdown => {},
    }

    info!("Terminating...");
}
