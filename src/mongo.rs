use crate::error;
use mongodb::options::ClientOptions;
use mongodb::{Client, Collection};

pub struct GlyphMongo {
    client: Client,
}

impl GlyphMongo {
    pub async fn new() -> error::Result<Self> {
        let conn_str = std::env::var("MONGO_CONN_STR").expect("Missing MONGO_CONN_STR");
        let client_options = ClientOptions::parse(conn_str)
            .await?;
        let client = Client::with_options(client_options)?;
        Ok(Self { client })
    }

    pub async fn collection<T>(&self, database: &str, collection: &str) -> Collection<T>
    where
        T: Send + Sync,
    {
        let db = self.client.database(database);
        db.collection::<T>(collection)
    }
}

// Database and collection names

pub const AUTH_DB: &str = "auth";
pub const ACCESS_TKN_COLL: &str = "access";
pub const EXCHANGE_CODE_COLL: &str = "exchange";
pub const REFRESH_TKN_COLL: &str = "refresh";

pub const PROFILE_DB: &str = "profile";
pub const ATHENA_COLL: &str = "athena";
pub const COMMON_CORE_COLL: &str = "common_core";
pub const COMMON_PUB_COLL: &str = "common_public";

pub const USER_DB: &str = "user";
pub const USERS_COLL: &str = "user";
pub const FRIENDS_COLL: &str = "friend";
