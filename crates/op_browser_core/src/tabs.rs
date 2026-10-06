use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TabId(u64);

impl TabId {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabLifecycle {
    Active,
    Background,
    Throttled,
    Frozen,
    Discarded,
    Restoring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardReason {
    MemoryPressure,
    UserRequest,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TabProtection {
    pub audible: bool,
    pub media_capture: bool,
    pub file_transfer: bool,
    pub unsaved_form: bool,
    pub pinned: bool,
}

impl TabProtection {
    pub fn prevents_automatic_discard(self) -> bool {
        self.audible || self.media_capture || self.file_transfer || self.unsaved_form || self.pinned
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabRestoreState {
    pub address: String,
    pub scroll_y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub address: String,
    pub lifecycle: TabLifecycle,
    pub protection: TabProtection,
    pub estimated_private_bytes: u64,
    pub last_active_tick: u64,
    pub restore: Option<TabRestoreState>,
}

#[derive(Debug, Default)]
pub struct TabManager {
    next_id: u64,
    clock: u64,
    active: Option<TabId>,
    tabs: Vec<Tab>,
    index: HashMap<TabId, usize>,
}

impl TabManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active_id(&self) -> Option<TabId> {
        self.active
    }

    pub fn active(&self) -> Option<&Tab> {
        self.get(self.active?)
    }

    pub fn get(&self, id: TabId) -> Option<&Tab> {
        self.tabs.get(*self.index.get(&id)?)
    }

    pub fn open(&mut self, address: impl Into<String>, activate: bool) -> TabId {
        self.clock = self.clock.saturating_add(1);
        self.next_id = self.next_id.saturating_add(1);
        let id = TabId(self.next_id);

        if activate {
            self.demote_active();
        }

        let lifecycle = if activate || self.active.is_none() {
            TabLifecycle::Active
        } else {
            TabLifecycle::Background
        };
        let tab = Tab {
            id,
            title: String::new(),
            address: address.into(),
            lifecycle,
            protection: TabProtection::default(),
            estimated_private_bytes: 0,
            last_active_tick: self.clock,
            restore: None,
        };
        self.index.insert(id, self.tabs.len());
        self.tabs.push(tab);

        if lifecycle == TabLifecycle::Active {
            self.active = Some(id);
        }
        id
    }

    pub fn activate(&mut self, id: TabId) -> bool {
        let Some(index) = self.index.get(&id).copied() else {
            return false;
        };
        if self.active == Some(id) {
            return true;
        }

        self.clock = self.clock.saturating_add(1);
        self.demote_active();
        let tab = &mut self.tabs[index];
        tab.lifecycle = if tab.lifecycle == TabLifecycle::Discarded {
            TabLifecycle::Restoring
        } else {
            TabLifecycle::Active
        };
        tab.last_active_tick = self.clock;
        self.active = Some(id);
        true
    }

    pub fn close(&mut self, id: TabId) -> bool {
        let Some(index) = self.index.remove(&id) else {
            return false;
        };
        let was_active = self.active == Some(id);
        self.tabs.remove(index);
        self.rebuild_index();

        if was_active {
            self.active = None;
            if let Some(candidate) = self.tabs.last().map(|tab| tab.id) {
                self.activate(candidate);
            }
        }
        true
    }

    pub fn update_navigation(
        &mut self,
        id: TabId,
        address: impl Into<String>,
        title: impl Into<String>,
    ) -> bool {
        let Some(tab) = self.get_mut(id) else {
            return false;
        };
        tab.address = address.into();
        tab.title = title.into();
        tab.restore = None;
        true
    }

    pub fn set_protection(&mut self, id: TabId, protection: TabProtection) -> bool {
        let Some(tab) = self.get_mut(id) else {
            return false;
        };
        tab.protection = protection;
        true
    }

    pub fn set_estimated_private_bytes(&mut self, id: TabId, bytes: u64) -> bool {
        let Some(tab) = self.get_mut(id) else {
            return false;
        };
        tab.estimated_private_bytes = bytes;
        true
    }

    pub fn set_background_state(&mut self, id: TabId, lifecycle: TabLifecycle) -> bool {
        if !matches!(
            lifecycle,
            TabLifecycle::Background | TabLifecycle::Throttled | TabLifecycle::Frozen
        ) || self.active == Some(id)
        {
            return false;
        }
        let Some(tab) = self.get_mut(id) else {
            return false;
        };
        if tab.lifecycle == TabLifecycle::Discarded || tab.lifecycle == TabLifecycle::Restoring {
            return false;
        }
        tab.lifecycle = lifecycle;
        true
    }

    pub fn automatic_discard_candidate(&self) -> Option<TabId> {
        self.tabs
            .iter()
            .filter(|tab| {
                self.active != Some(tab.id)
                    && !tab.protection.prevents_automatic_discard()
                    && matches!(
                        tab.lifecycle,
                        TabLifecycle::Background | TabLifecycle::Throttled | TabLifecycle::Frozen
                    )
            })
            .max_by_key(|tab| {
                (
                    tab.estimated_private_bytes,
                    u64::MAX.saturating_sub(tab.last_active_tick),
                )
            })
            .map(|tab| tab.id)
    }

    pub fn discard(&mut self, id: TabId, reason: DiscardReason, scroll_y: i32) -> bool {
        if self.active == Some(id) {
            return false;
        }
        let Some(tab) = self.get_mut(id) else {
            return false;
        };
        if reason == DiscardReason::MemoryPressure && tab.protection.prevents_automatic_discard() {
            return false;
        }
        tab.restore = Some(TabRestoreState {
            address: tab.address.clone(),
            scroll_y,
        });
        tab.lifecycle = TabLifecycle::Discarded;
        tab.estimated_private_bytes = 0;
        true
    }

    pub fn finish_restore(&mut self, id: TabId) -> bool {
        let is_active = self.active == Some(id);
        let Some(tab) = self.get_mut(id) else {
            return false;
        };
        if tab.lifecycle != TabLifecycle::Restoring {
            return false;
        }
        tab.lifecycle = if is_active {
            TabLifecycle::Active
        } else {
            TabLifecycle::Background
        };
        true
    }

    fn get_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        let index = *self.index.get(&id)?;
        self.tabs.get_mut(index)
    }

    fn demote_active(&mut self) {
        let Some(id) = self.active.take() else {
            return;
        };
        if let Some(tab) = self.get_mut(id)
            && matches!(
                tab.lifecycle,
                TabLifecycle::Active | TabLifecycle::Restoring
            )
        {
            tab.lifecycle = TabLifecycle::Background;
        }
    }

    fn rebuild_index(&mut self) {
        self.index.clear();
        for (index, tab) in self.tabs.iter().enumerate() {
            self.index.insert(tab.id, index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_activate_close_keeps_one_canonical_active_tab() {
        let mut manager = TabManager::new();
        let first = manager.open("https://one.example", true);
        let second = manager.open("https://two.example", false);
        assert_eq!(manager.active_id(), Some(first));
        assert_eq!(
            manager.get(second).unwrap().lifecycle,
            TabLifecycle::Background
        );

        assert!(manager.activate(second));
        assert_eq!(
            manager.get(first).unwrap().lifecycle,
            TabLifecycle::Background
        );
        assert_eq!(manager.get(second).unwrap().lifecycle, TabLifecycle::Active);

        assert!(manager.close(second));
        assert_eq!(manager.active_id(), Some(first));
        assert_eq!(manager.get(first).unwrap().lifecycle, TabLifecycle::Active);
    }

    #[test]
    fn automatic_discard_prefers_large_unprotected_background_tab() {
        let mut manager = TabManager::new();
        let active = manager.open("about:start", true);
        let small = manager.open("https://small.example", false);
        let large = manager.open("https://large.example", false);
        let protected = manager.open("https://audio.example", false);
        manager.set_estimated_private_bytes(small, 10);
        manager.set_estimated_private_bytes(large, 1000);
        manager.set_estimated_private_bytes(protected, 5000);
        manager.set_protection(
            protected,
            TabProtection {
                audible: true,
                ..TabProtection::default()
            },
        );

        assert_eq!(manager.automatic_discard_candidate(), Some(large));
        assert_ne!(manager.automatic_discard_candidate(), Some(active));
    }

    #[test]
    fn discard_retains_restore_state_and_activation_requests_restore() {
        let mut manager = TabManager::new();
        let _active = manager.open("about:start", true);
        let background = manager.open("https://docs.example/page", false);
        assert!(manager.discard(background, DiscardReason::MemoryPressure, 420));
        let discarded = manager.get(background).unwrap();
        assert_eq!(discarded.lifecycle, TabLifecycle::Discarded);
        assert_eq!(discarded.restore.as_ref().unwrap().scroll_y, 420);

        assert!(manager.activate(background));
        assert_eq!(
            manager.get(background).unwrap().lifecycle,
            TabLifecycle::Restoring
        );
        assert!(manager.finish_restore(background));
        assert_eq!(
            manager.get(background).unwrap().lifecycle,
            TabLifecycle::Active
        );
    }

    #[test]
    fn protected_tabs_require_explicit_user_discard() {
        let mut manager = TabManager::new();
        let _active = manager.open("about:start", true);
        let tab = manager.open("https://form.example", false);
        manager.set_protection(
            tab,
            TabProtection {
                unsaved_form: true,
                ..TabProtection::default()
            },
        );
        assert!(!manager.discard(tab, DiscardReason::MemoryPressure, 0));
        assert!(manager.discard(tab, DiscardReason::UserRequest, 0));
    }
}
