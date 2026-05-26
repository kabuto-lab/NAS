//! Cross-crate newtype IDs · ENTITY §15 (multi-tenancy contract).
//!
//! Every ID is a `#[repr(transparent)]` newtype around `Uuid` so that:
//!   - serde representations are identical to the raw UUID;
//!   - the type-system makes accidental cross-ID assignment a compile error.
//!
//! UUID version policy:
//!   - `TenantId`, `SiteId`, `UserId`, `RoleId`, `PostId`, `MediaId`
//!     → **v4** (random). Time-leaking would be a multi-tenant privacy
//!     issue (cf. ENTITY §15).
//!   - `RequestId` → **v7** (time-ordered) for log-correlation and
//!     better B-tree locality if ever indexed.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        #[repr(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            #[must_use]
            pub fn new_v4() -> Self {
                Self(Uuid::new_v4())
            }

            #[must_use]
            pub const fn from_uuid(u: Uuid) -> Self {
                Self(u)
            }

            #[must_use]
            pub const fn into_uuid(self) -> Uuid {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0.simple())
            }
        }

        impl From<Uuid> for $name {
            fn from(u: Uuid) -> Self {
                Self(u)
            }
        }
    };
}

id_newtype!(TenantId, "Tenant boundary — see ENTITY §15.");
id_newtype!(SiteId, "Site within a tenant.");
id_newtype!(UserId, "User within a tenant.");
id_newtype!(RoleId, "Role within a tenant.");
id_newtype!(PostId, "Post within a site.");
id_newtype!(MediaId, "Media asset within a tenant.");

/// Per-request correlation id. UUID v7 for time-ordering &
/// log-rotation friendliness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct RequestId(pub Uuid);

impl RequestId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    #[must_use]
    pub const fn from_uuid(u: Uuid) -> Self {
        Self(u)
    }

    #[must_use]
    pub const fn into_uuid(self) -> Uuid {
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
        write!(f, "{}", self.0.simple())
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_methods
)]
mod tests {
    use super::*;

    #[test]
    fn tenant_id_serde_roundtrip() {
        let id = TenantId::new_v4();
        let json = serde_json::to_string(&id).expect("serialize");
        let back: TenantId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, back);
    }

    #[test]
    fn request_id_is_time_ordered() {
        let a = RequestId::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = RequestId::new();
        assert!(a < b, "v7 should be time-ordered: {a} < {b}");
    }

    #[test]
    fn request_id_default_uses_now_v7() {
        let a = RequestId::default();
        // sanity: default and new produce v7 (version field == 7)
        let u = a.into_uuid();
        let bytes = u.as_bytes();
        // UUID version is in the high nibble of byte 6
        let version = (bytes[6] >> 4) & 0x0F;
        assert_eq!(version, 7, "RequestId must be UUID v7");
    }

    #[test]
    fn tenant_id_display_is_simple_uuid_form() {
        let u = Uuid::new_v4();
        let id = TenantId::from_uuid(u);
        let rendered = format!("{id}");
        assert_eq!(rendered.len(), 32, "simple form has 32 hex chars");
        assert!(
            !rendered.contains('-'),
            "simple form has no hyphens: {rendered}"
        );
    }

    #[test]
    fn ids_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TenantId>();
        assert_send_sync::<SiteId>();
        assert_send_sync::<UserId>();
        assert_send_sync::<RoleId>();
        assert_send_sync::<PostId>();
        assert_send_sync::<MediaId>();
        assert_send_sync::<RequestId>();
    }

    #[test]
    fn id_from_uuid_roundtrip() {
        let u = Uuid::new_v4();
        let id = TenantId::from_uuid(u);
        assert_eq!(id.into_uuid(), u);
    }

    #[test]
    fn id_via_from_trait() {
        let u = Uuid::new_v4();
        let id: SiteId = u.into();
        assert_eq!(id.into_uuid(), u);
    }

    #[test]
    fn distinct_ids_are_distinct_types() {
        // Compile-time test: TenantId and SiteId cannot be mixed.
        // Both are `From<Uuid>` so the conversion must be explicit.
        let u = Uuid::new_v4();
        let tenant = TenantId::from(u);
        let site = SiteId::from(u);
        // Their underlying UUIDs match, but the types themselves differ
        // (this assertion is trivial — the type-distinction is enforced
        // at compile time by the absence of `From<TenantId> for SiteId`).
        assert_eq!(tenant.into_uuid(), site.into_uuid());
    }

    #[test]
    fn id_is_copy() {
        let id = TenantId::new_v4();
        let copy = id;
        assert_eq!(id, copy);
    }
}
