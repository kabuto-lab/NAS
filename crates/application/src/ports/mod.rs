//! Port traits — interfaces реализуемые infrastructure adapters.

pub mod cms;
pub mod tenant;

pub use cms::CmsRepository;
pub use tenant::TenantResolver;
