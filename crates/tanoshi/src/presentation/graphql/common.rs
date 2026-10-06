use async_graphql::{SimpleObject, connection::CursorType, scalar};
use base64::{Engine, engine::general_purpose};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tanoshi_lib::prelude::Input;

use crate::domain::entities::history::HistoryCursor;

pub struct Cursor(pub i64, pub i64);

impl CursorType for Cursor {
    type Error = anyhow::Error;

    fn decode_cursor(s: &str) -> Result<Self, Self::Error> {
        let cursor = String::from_utf8(general_purpose::STANDARD.decode(s)?)?;
        let decoded = cursor.split('#').collect::<Vec<&str>>();
        let timestamp = decoded[0].parse()?;
        let id = decoded[1].parse()?;
        Ok(Self(timestamp, id))
    }

    fn encode_cursor(&self) -> String {
        general_purpose::STANDARD.encode(format!("{}#{}", self.0, self.1))
    }
}

impl CursorType for HistoryCursor {
    type Error = anyhow::Error;

    fn decode_cursor(s: &str) -> Result<Self, Self::Error> {
        let decoded = String::from_utf8(general_purpose::STANDARD.decode(s)?)?;
        let parts: Vec<_> = decoded.split('#').collect();
        let (read_at, id) = match parts.as_slice() {
            ["history", timestamp, id] => (
                NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S%.f")?,
                *id,
            ),
            // Accept cursors issued before history acquired full precision.
            [timestamp, id] => (
                chrono::DateTime::from_timestamp(timestamp.parse()?, 0)
                    .ok_or_else(|| anyhow::anyhow!("invalid history cursor timestamp"))?
                    .naive_utc(),
                *id,
            ),
            _ => anyhow::bail!("invalid history cursor"),
        };
        Ok(Self {
            read_at,
            manga_id: id.parse()?,
        })
    }

    fn encode_cursor(&self) -> String {
        general_purpose::STANDARD.encode(format!(
            "history#{}#{}",
            self.read_at.format("%Y-%m-%d %H:%M:%S%.9f"),
            self.manga_id,
        ))
    }
}

#[derive(Debug, Clone, SimpleObject)]
pub struct ReadProgress {
    pub at: NaiveDateTime,
    pub last_page: i64,
    pub is_complete: bool,
}

#[derive(Deserialize, Serialize)]
pub struct InputList(pub Vec<Input>);

scalar!(InputList);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_cursors_preserve_nanoseconds_and_accept_legacy_cursors() {
        let cursor = HistoryCursor {
            read_at: NaiveDateTime::parse_from_str(
                "2026-09-10 12:00:00.123456789",
                "%Y-%m-%d %H:%M:%S%.f",
            )
            .unwrap(),
            manga_id: 42,
        };
        assert_eq!(
            HistoryCursor::decode_cursor(&cursor.encode_cursor()).unwrap(),
            cursor
        );
        let legacy = Cursor(cursor.read_at.and_utc().timestamp(), 42).encode_cursor();
        let decoded = HistoryCursor::decode_cursor(&legacy).unwrap();
        assert_eq!(decoded.manga_id, 42);
        assert_eq!(
            decoded.read_at.and_utc().timestamp(),
            cursor.read_at.and_utc().timestamp()
        );
        assert_eq!(decoded.read_at.and_utc().timestamp_subsec_nanos(), 0);
    }

    #[test]
    fn malformed_history_cursors_return_errors() {
        assert!(HistoryCursor::decode_cursor("invalid base64").is_err());
        for payload in [
            "",
            "1",
            "1#2#3",
            "history#not-a-date#1",
            "history#2026-01-01 00:00:00#bad-id",
            "9223372036854775807#1",
        ] {
            assert!(
                HistoryCursor::decode_cursor(&general_purpose::STANDARD.encode(payload)).is_err(),
                "{payload}"
            );
        }
    }
}
