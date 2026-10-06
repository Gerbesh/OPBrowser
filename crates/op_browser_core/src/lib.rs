//! UI-independent browser-product state.
//!
//! The browser process owns tabs and lifecycle policy. Renderers consume commands derived from
//! this state, but must not own the canonical tab list.

mod tabs;

pub use tabs::{
    DiscardReason, Tab, TabId, TabLifecycle, TabManager, TabProtection, TabRestoreState,
};
