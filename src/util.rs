use std::fmt;
use crate::error;
use axum::http::HeaderMap;
use base64::Engine;
use serde::de::Visitor;
use serde::{de, Deserializer};
use uuid::Uuid;

pub trait UuidString {
    fn to_stripped_string(&self) -> String;
}

impl UuidString for Uuid {
    fn to_stripped_string(&self) -> String {
        self.to_string().replace("-", "")
    }
}
fn invalid_auth<T>(_: T) -> error::Error {
    error::Error::InvalidAuthorizationHeader
}

pub fn extract_client_id(headers: &HeaderMap) -> error::Result<String> {
    let auth_str = headers.get("Authorization").unwrap().to_str().map_err(invalid_auth)?;
    let decoded = base64::engine::general_purpose::STANDARD.decode(auth_str).map_err(invalid_auth)?;
    let client_id = std::str::from_utf8(&*decoded).map_err(invalid_auth)?;
    let split = client_id.split(':');

    let coll = split.collect::<Vec<&str>>();
    if coll.len() == 2 {
        Ok(coll[0].to_string())
    } else {
        Err(error::Error::InvalidAuthorizationHeader)
    }
}

pub fn deserialize_option_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionStringVisitor;

    impl<'de> Visitor<'de> for OptionStringVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an Option<String> that can be 'null', 'None', or a string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match value {
                "null" | "None" => Ok(None),
                _ => Ok(Some(value.to_string())),
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_str(self)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_option(OptionStringVisitor)
}

