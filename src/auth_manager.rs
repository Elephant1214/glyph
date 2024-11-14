use crate::mongo::{GlyphMongo, ACCESS_TKN_COLL, AUTH_DB, EXCHANGE_CODE_COLL, REFRESH_TKN_COLL};
use crate::route::account::auth::GrantType;
use crate::user::User;
use crate::util::UuidString;
use crate::error;
use chrono::{DateTime, TimeDelta, Utc};
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use mongodb::bson::doc;
use mongodb::Collection;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::ops::Add;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct OAuthToken {
    pub(crate) token: String,
    pub(crate) account_id: Uuid,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime", rename = "expireAt")]
    pub(crate) expires_at: DateTime<Utc>,
    #[serde(rename = "expireAfterSeconds")]
    expire_after_seconds: i64,
}

pub struct OAuthManager {
    signing_key: Hmac<Sha256>,
}

const P_CLAIM: &str = "eNqtk8lOAzEMht+nQkhlO1iaA0tBnEBC4joyiWdqNeNUiVPo2+NhGShLBxCnbF7+/0vSxKTCSuBCLD5rTNgS5HVW6uCcUEsif5kDij/Y5aexmh7tND9P2x9NK5kSuCgNt9Xe9tK3i5lnPSPPDpX8DaUVpQvsaJeFR4XdLq4DrrdkY9NwYDuDZbkL7GDYGBE2pvtfFc+kZRnyAxZxcyPo472EiB4CrwgmJilHxxhACbsMkqGR6mjHCmGILeQ52h1BbBpK2YI/15nA+Yu20yhKoieFg+9j0blYRAdKz8tq+isImzZeS0YsOgd60Pppck+tsYKEHOpMOXOUWtktSKvD7d16AFQCakK3YBkM2w5lxTYRdebps5u2YPKMks3P10Z7eZQEw7FJvJKwtsgPWCfv6r6M2YB2pOgtEubLun/2Nft6maL/07vfBPiLLzl99yVNxIodsRgTcfQNtAHX3doa97cwpniz495bx0dUELK5";

impl OAuthManager {
    pub fn new(
        signing_key: Hmac<Sha256>,
    ) -> Self {
        Self { signing_key }
    }

    /// Generated an HMAC SHA256 key for signing JWT tokens with the supplied UUID.
    pub(crate) fn gen_signing_key(
        runner_id: Uuid,
    ) -> error::Result<Hmac<Sha256>> {
        let key: Hmac<Sha256> = Hmac::new_from_slice(runner_id.to_stripped_string().as_bytes())?;
        Ok(key.clone())
    }

    /// Signs, stores, then returns a refresh token for the supplied [User] to be passed to the
    /// client on startup. This is used instead of the game's password auth because users use
    /// Discord to sign in.
    pub async fn make_exchange_code(
        &self,
        mongo: &GlyphMongo,
        user: &User,
        expires_at: Option<i64>,
    ) -> error::Result<OAuthToken> {
        let expires_at = expires_at.unwrap_or(28800);
        let expiration = Utc::now().add(TimeDelta::seconds(expires_at));

        let mut claims = BTreeMap::new();
        claims.insert("srvc", "glyph");
        let user_id = user.account_id.to_stripped_string();
        claims.insert("userId", user_id.as_str());
        let jti = Uuid::new_v4().to_stripped_string();
        claims.insert("jti", jti.as_str());
        let exp = expiration.timestamp().to_string();
        claims.insert("exp", exp.as_str());

        let token = claims.sign_with_key(&self.signing_key)?;
        let oauth_token = OAuthToken {
            token,
            account_id: user.account_id,
            expires_at: expiration,
            expire_after_seconds: 0,
        };

        mongo.collection::<OAuthToken>(AUTH_DB, EXCHANGE_CODE_COLL).await.insert_one(&oauth_token).await?;
        Ok(oauth_token)
    }

    /// See [OAuthManager::get_token]
    pub async fn get_exchange_code(mongo: &GlyphMongo, token: &String) -> error::Result<Option<OAuthToken>> {
        OAuthManager::get_token(mongo.collection::<OAuthToken>(AUTH_DB, EXCHANGE_CODE_COLL).await, token).await
    }

    /// See [OAuthManager::kill_token]
    pub async fn kill_exchange_code(mongo: &GlyphMongo, token: &String) -> error::Result<bool> {
        OAuthManager::kill_token(mongo.collection::<OAuthToken>(AUTH_DB, EXCHANGE_CODE_COLL).await, token).await
    }

    /// Signs and returns a client token. This is just used to tell the client it can actually play
    /// the game and is pretty much immediately thrown away by the client as far as I know.
    pub fn make_client_token(
        &self,
        client_id: &str,
    ) -> error::Result<String> {
        let expiration = Utc::now().add(TimeDelta::hours(4));

        let mut claims = BTreeMap::new();
        claims.insert("p", P_CLAIM);
        claims.insert("clsvc", "prod-fn");
        claims.insert("t", "s");
        claims.insert("mver", "false");
        claims.insert("clid", client_id);
        claims.insert("ic", "true");
        let exp = expiration.timestamp().to_string();
        claims.insert("exp", exp.as_str());
        claims.insert("am", "client_credentials");
        let iat = expiration.timestamp().to_string();
        claims.insert("iat", iat.as_str());
        let jti = Uuid::new_v4().to_stripped_string();
        claims.insert("jti", jti.as_str());
        claims.insert("pfpid", "prod-fn");

        let token = claims.sign_with_key(&self.signing_key)?;
        Ok(token)
    }

    /// Signs, stores, then returns an access token for the supplied [User].
    pub async fn make_access_token(
        &self,
        mongo: &GlyphMongo,
        user: &User,
        client_id: &str,
        device_id: &str,
        grant_type: &GrantType,
        expires_in: Option<i64>,
    ) -> error::Result<OAuthToken> {
        let expires_in = expires_in.unwrap_or(7200);
        let expiration = Utc::now().add(TimeDelta::seconds(expires_in));

        let mut claims = BTreeMap::new();
        claims.insert("app", "fortnite");
        let account_id = user.account_id.to_stripped_string();
        claims.insert("sub", account_id.as_str());
        claims.insert("dvid", device_id);
        claims.insert("mver", "false");
        claims.insert("clid", client_id);
        claims.insert("dn", user.display_name.as_str());
        let am = grant_type.to_string();
        claims.insert("am", am.as_str());
        claims.insert("p", P_CLAIM);
        claims.insert("iai", account_id.as_str());
        claims.insert("sec", "1");
        claims.insert("clsvc", "prod-fn");
        claims.insert("t", "s");
        claims.insert("ic", "true");
        let jti = Uuid::new_v4().to_stripped_string();
        claims.insert("jti", jti.as_str());
        let creation_date = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        claims.insert("creation_date", creation_date.as_str());
        claims.insert("hours_expire", "2");
        let exp = expiration.timestamp().to_string();
        claims.insert("exp", exp.as_str());

        let token = claims.sign_with_key(&self.signing_key)?;
        let oauth_token = OAuthToken {
            token,
            account_id: user.account_id,
            expires_at: expiration,
            expire_after_seconds: 0,
        };

        mongo.collection::<OAuthToken>(AUTH_DB, ACCESS_TKN_COLL).await.insert_one(&oauth_token).await?;
        Ok(oauth_token)
    }

    /// See [OAuthManager::get_token]
    pub async fn get_access_token(mongo: &GlyphMongo, token: &str) -> error::Result<Option<OAuthToken>> {
        OAuthManager::get_token(mongo.collection::<OAuthToken>(AUTH_DB, ACCESS_TKN_COLL).await, token).await
    }

    /// See [OAuthManager::kill_token]
    pub async fn kill_access_token(mongo: &GlyphMongo, token: &str) -> error::Result<bool> {
        OAuthManager::kill_token(mongo.collection::<OAuthToken>(AUTH_DB, ACCESS_TKN_COLL).await, token).await
    }

    /// Signs, stores, then returns a refresh token for the supplied [User].
    pub async fn make_refresh_token(
        &self,
        mongo: &GlyphMongo,
        user: &User,
        client_id: &str,
        device_id: &str,
        grant_type: &GrantType,
        expires_at: Option<i64>,
    ) -> error::Result<OAuthToken> {
        let expires_at = expires_at.unwrap_or(28800);
        let expiration = Utc::now().add(TimeDelta::seconds(expires_at));

        let mut claims = BTreeMap::new();
        let account_id = user.account_id.to_stripped_string();
        claims.insert("sub", account_id.as_str());
        claims.insert("dvid", device_id);
        claims.insert("t", "s");
        claims.insert("clid", client_id);
        let am = grant_type.to_string();
        claims.insert("am", am.as_str());
        let jti = Uuid::new_v4().to_stripped_string();
        claims.insert("jti", jti.as_str());
        let creation_date = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        claims.insert("creation_date", creation_date.as_str());
        claims.insert("hours_expire", "2");
        let exp = expiration.timestamp().to_string();
        claims.insert("exp", exp.as_str());

        let token = claims.sign_with_key(&self.signing_key)?;
        let oauth_token = OAuthToken {
            token,
            account_id: user.account_id,
            expires_at: expiration,
            expire_after_seconds: 0,
        };

        mongo.collection::<OAuthToken>(AUTH_DB, REFRESH_TKN_COLL).await.insert_one(&oauth_token).await?;
        Ok(oauth_token)
    }

    /// See [OAuthManager::get_token]
    pub async fn get_refresh_token(mongo: &GlyphMongo, token: &str) -> error::Result<Option<OAuthToken>> {
        OAuthManager::get_token(mongo.collection::<OAuthToken>(AUTH_DB, REFRESH_TKN_COLL).await, token).await
    }

    /// See [OAuthManager::kill_token]
    pub async fn kill_refresh_token(mongo: &GlyphMongo, token: &str) -> error::Result<bool> {
        OAuthManager::kill_token(mongo.collection::<OAuthToken>(AUTH_DB, ACCESS_TKN_COLL).await, token).await
    }

    /// Kills all exchange codes, access tokens, and refresh tokens for the supplied [User].
    pub async fn kill_user_tokens(mongo: &GlyphMongo, user: &User) -> error::Result<bool> {
        let filter = doc! { "account_id": user.account_id.to_string() };
        mongo.collection::<OAuthToken>(AUTH_DB, EXCHANGE_CODE_COLL).await.delete_many(filter.clone()).await?;
        mongo.collection::<OAuthToken>(AUTH_DB, ACCESS_TKN_COLL).await.delete_many(filter.clone()).await?;
        mongo.collection::<OAuthToken>(AUTH_DB, REFRESH_TKN_COLL).await.delete_many(filter).await?;
        Ok(true)
    }

    /// Returns an Ok [error::Result] of true if a document was found and returned, otherwise an Ok
    /// result of false if no document was found. If Mongo encounters an error for some reason, an
    /// Err of [error::Error::MongoError] is returned.
    async fn get_token(collection: Collection<OAuthToken>, token: &str) -> error::Result<Option<OAuthToken>> {
        let filter = doc! { "token": token };
        let option = collection.find_one(filter).await?;
        Ok(option)
    }

    /// Returns an Ok [error::Result] of true if one document was found and deleted, otherwise an Ok
    /// result of false if none were deleted. If Mongo encounters an error for some reason, an Err
    /// of [error::Error::MongoError] is returned.
    async fn kill_token(collection: Collection<OAuthToken>, token: &str) -> error::Result<bool> {
        let filter = doc! { "token": token };
        let result = collection.delete_one(filter).await?;
        Ok(result.deleted_count == 1)
    }
}
