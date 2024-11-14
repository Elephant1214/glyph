pub fn serialize_datetime<S>(
    date: &chrono::DateTime<chrono::Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error> where
    S: serde::Serializer,
{
    let timestamp = date.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    serializer.serialize_str(&timestamp)
}