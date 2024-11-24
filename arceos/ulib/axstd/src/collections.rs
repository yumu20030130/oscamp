//! Collections module

#[cfg(feature = "alloc")]
pub use alloc::collections::*;

#[cfg(feature = "alloc")]
pub use hashbrown::HashMap;