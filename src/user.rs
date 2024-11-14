use crate::error;
use crate::error::Error;
use crate::mongo::{GlyphMongo, USERS_COLL, USER_DB};
use bson::doc;
use chrono::{DateTime, Utc};
use mongodb::Collection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub enum Platform {
    WeGame,
    EpicPCKorea,
    Epic,
    EpicPc,
    EpicAndroid,
    PSN,
    Live,
    IOSAppStore,
    Nintendo,
    Samsung,
    GooglePlayer,
    Shared,
}

#[derive(Serialize, Deserialize)]
pub struct DisplayNameHistory {
    display_name: String,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    changed_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    pub(crate) account_id: uuid::Uuid,
    pub(crate) display_name: String,
    pub(crate) banned: bool,
    pub(crate) discord_id: u64,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub(crate) last_login: DateTime<Utc>,
    pub(crate) platform: Platform,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub(crate) created: DateTime<Utc>,
    pub(crate) name_history: Vec<DisplayNameHistory>,
}

async fn new_unused_uuid(coll: &Collection<User>) -> error::Result<Uuid> {
    loop {
        let new_uuid = Uuid::new_v4();
        let filter = doc! { "account_id": new_uuid.to_string() };

        match coll.find_one(filter).await {
            Ok(Some(_)) => continue,
            Ok(None) => return Ok(new_uuid),
            Err(e) => return Err(Error::MongoError(e))
        }
    }
}

pub async fn create_user(mongo: &GlyphMongo, discord_id: u64, display_name: String) -> error::Result<User> {
    let user_collection = mongo.collection::<User>(USER_DB, USERS_COLL).await;

    let uuid = new_unused_uuid(&user_collection).await?;
    let now = Utc::now();
    let user = User {
        account_id: uuid,
        display_name,
        banned: false,
        discord_id,
        last_login: now,
        platform: Platform::EpicPc,
        created: now,
        name_history: vec![],
    };

    user_collection.insert_one(&user).await?;
    Ok(user)
}

pub async fn get_user(mongo: &GlyphMongo, account_id: &Uuid) -> error::Result<Option<User>> {
    let user_collection = mongo.collection::<User>(USER_DB, USERS_COLL).await;
    let found = user_collection.find_one(doc! { "account_id": account_id.to_string() }).await?;
    Ok(found)
}
