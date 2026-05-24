//! Port traits — interfaces реализуемые infrastructure adapters.

pub mod cms;
pub mod tenant;
pub mod user;

pub use cms::CmsRepository;
pub use tenant::TenantResolver;
pub use user::UserRepository;
