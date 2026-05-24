//! CMS domain — pages, blocks, ED editor types.

pub mod aggregate;
pub mod blocks;
pub mod value_objects;

pub use aggregate::PublishedPage;
pub use blocks::{Block, CanvasElement, Column, ElStyle, Section, Widget};
pub use value_objects::{PageLocale, PageSlug, PageStatus};
