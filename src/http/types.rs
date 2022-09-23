use std::fmt::Formatter;

use serde::{Deserialize, Deserializer, ser, Serialize, Serializer};
use serde::de::Visitor;
use time::{format_description, OffsetDateTime};

#[derive(sqlx::Type)]
pub struct Timestamp(pub OffsetDateTime);

impl Timestamp {
    pub fn now() -> Self {
        Self(OffsetDateTime::now_utc())
    }
}

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let format = format_description::parse(
            "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
         sign:mandatory]:[offset_minute]:[offset_second]",
        ).map_err(|e| ser::Error::custom(e.to_string()))?;
        let s =  self.0.format(&format).map_err(|e|ser::Error::custom(e.to_string()))?;
        serializer.collect_str(&s)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        struct StrVisitor;
        impl Visitor<'_> for StrVisitor {
            type Value = Timestamp;

            fn expecting(&self, f: &mut Formatter) -> std::fmt::Result {
                f.pad("expected string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
            {
                let format =  format_description::parse(
                    "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
     sign:mandatory]:[offset_minute]:[offset_second]",
                ).map_err(|e|serde::de::Error::custom(e.to_string()))?;
               let time = OffsetDateTime::parse(   v, &format)
               .map_err(|e|serde::de::Error::custom(e.to_string()))?;
               Ok(Timestamp(time))
            }
        }
        deserializer.deserialize_str(StrVisitor)
    }
}
