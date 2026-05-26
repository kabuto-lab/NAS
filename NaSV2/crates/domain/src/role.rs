//! Role aggregate · ENTITY §14.
//!
//! A role is a named bundle of capabilities scoped to a tenant. Users
//! get roles assigned; their effective capabilities = union of role
//! capabilities (computed in `nas2-application`, not here).

use chrono::{DateTime, Utc};
use nas2_common::{RoleId, TenantId};
use serde::{Deserialize, Serialize};

use crate::capability::{Capability, CapabilitySet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Role {
    pub id: RoleId,
    pub tenant_id: TenantId,
    pub name: String,
    pub capabilities: CapabilitySet,
    pub created_at: DateTime<Utc>,
}

impl Role {
    #[must_use]
    pub fn has(&self, c: Capability) -> bool {
        self.capabilities.contains(c)
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
    use chrono::TimeZone;

    fn role_with(caps: &[Capability]) -> Role {
        let mut set = CapabilitySet::new();
        for c in caps {
            set.insert(*c);
        }
        Role {
            id: RoleId::new_v4(),
            tenant_id: TenantId::new_v4(),
            name: "editor".to_owned(),
            capabilities: set,
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
        }
    }

    #[test]
    fn has_capability_when_present() {
        let r = role_with(&[Capability::CmsPageDraft, Capability::CmsPageRead]);
        assert!(r.has(Capability::CmsPageDraft));
        assert!(r.has(Capability::CmsPageRead));
    }

    #[test]
    fn has_capability_returns_false_when_absent() {
        let r = role_with(&[Capability::CmsPageRead]);
        assert!(!r.has(Capability::ManageSite));
    }

    #[test]
    fn role_serde_roundtrip() {
        let r = role_with(&[Capability::CmsPageRead, Capability::MediaUpload]);
        let json = serde_json::to_string(&r).unwrap();
        let back: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }
}
