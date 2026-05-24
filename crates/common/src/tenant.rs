//! Tenant identity and context.
//!
//! `TenantId` — newtype wrapper над `Uuid`, **compile-time** защита от случайной
//! подстановки UserId / любого другого Uuid в tenant-scoped функцию (`ENTITY.md §3`).
//!
//! `TenantContext` — карта tenant'а в request scope: id + slug + status. Помечен
//! `Copy` чтобы избежать аллокаций на каждый запрос (`ENTITY.md §11.5` — TenantContext
//! не должен `.clone()`'аться).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Newtype-обёртка над `Uuid` для tenant identifier.
///
/// Все tenant-scoped функции принимают `TenantId` (или ссылку на содержащий
/// `TenantContext`). Это compile-time гарантия что `UserId` или произвольный `Uuid`
/// не может быть случайно подставлен.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TenantId(pub Uuid);

impl TenantId {
    #[must_use]
    pub const fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }

    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl std::fmt::Display for TenantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl From<Uuid> for TenantId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Статус тенанта. Только `Active` пропускается tenant-resolver middleware.
/// `Suspended` / `Archived` → 403 даже на `@Public()` endpoint (audit §5.2 #10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TenantStatus {
    Active,
    Pending,
    Suspended,
    Archived,
}

impl TenantStatus {
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Pending => "pending",
            Self::Suspended => "suspended",
            Self::Archived => "archived",
        }
    }
}

impl std::fmt::Display for TenantStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Tenant context per request. Внедряется `tenant_resolver` middleware
/// в `req.extensions::<TenantContext>()`.
///
/// `Copy + Clone` намеренно — `clone()` дешёвый (4 fields на стэке + Arc<str>
/// inside slug). См. `ENTITY.md §11.5` allocation budget.
#[derive(Clone, Debug)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub tenant_slug: std::sync::Arc<str>,
    pub status: TenantStatus,
    pub request_id: crate::ids::RequestId,
    pub user_id: Option<crate::ids::UserId>,
}

impl TenantContext {
    /// Construct для unit-тестов и mock'ов.
    #[must_use]
    pub fn new_for_test(tenant_id: TenantId, slug: &str) -> Self {
        Self {
            tenant_id,
            tenant_slug: std::sync::Arc::from(slug),
            status: TenantStatus::Active,
            request_id: crate::ids::RequestId::new(),
            user_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_id_display_matches_uuid() {
        let uuid = Uuid::nil();
        let tid = TenantId::new(uuid);
        assert_eq!(format!("{tid}"), format!("{uuid}"));
    }

    #[test]
    fn tenant_id_from_uuid_roundtrip() {
        let uuid = Uuid::new_v4();
        let tid: TenantId = uuid.into();
        assert_eq!(tid.into_inner(), uuid);
    }

    #[test]
    fn tenant_status_serde_roundtrip() {
        for status in [
            TenantStatus::Active,
            TenantStatus::Pending,
            TenantStatus::Suspended,
            TenantStatus::Archived,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: TenantStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn tenant_status_is_active_only_for_active() {
        assert!(TenantStatus::Active.is_active());
        assert!(!TenantStatus::Pending.is_active());
        assert!(!TenantStatus::Suspended.is_active());
        assert!(!TenantStatus::Archived.is_active());
    }
}
