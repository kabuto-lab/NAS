//! Bounded identifier newtypes.
//!
//! - [`UserId`] — Uuid newtype для пользователей (admin / staff / client roles).
//! - [`RequestId`] — Ulid для трассировки запросов через все spans + Sentry tags.

use serde::{Deserialize, Serialize};
use ulid::Ulid;
use uuid::Uuid;

/// User identifier (admin / staff / client).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(pub Uuid);

impl UserId {
    #[must_use]
    pub const fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl From<Uuid> for UserId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Request identifier — ULID для трассировки и Sentry correlation.
///
/// Генерируется middleware'ом `request_id` на каждый incoming HTTP-запрос
/// (или принимается из `X-Request-Id` header если client уже задал).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(pub Ulid);

impl RequestId {
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    #[must_use]
    pub const fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    #[must_use]
    pub fn into_inner(self) -> Ulid {
        self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::str::FromStr for RequestId {
    type Err = ulid::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ulid::from_string(s).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_id_unique() {
        let r1 = RequestId::new();
        let r2 = RequestId::new();
        assert_ne!(r1, r2);
    }

    #[test]
    fn request_id_string_roundtrip() {
        let r = RequestId::new();
        let s = r.to_string();
        let parsed: RequestId = s.parse().unwrap();
        assert_eq!(r, parsed);
    }

    #[test]
    fn user_id_display_matches_uuid() {
        let uuid = Uuid::new_v4();
        let uid = UserId::new(uuid);
        assert_eq!(format!("{uid}"), format!("{uuid}"));
    }
}
