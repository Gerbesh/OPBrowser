use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

impl NodeId {
    const SLOT_BITS: u32 = usize::BITS / 2;
    const SLOT_MASK: usize = (1usize << Self::SLOT_BITS) - 1;
    // Keep native handles below the synthetic VM ID range (upper half).
    const MAX_GENERATION: usize = (1usize << (usize::BITS - Self::SLOT_BITS - 1)) - 1;

    fn from_slot(slot: usize, generation: usize) -> Self {
        assert!(slot <= Self::SLOT_MASK && generation <= Self::MAX_GENERATION);
        Self((generation << Self::SLOT_BITS) | slot)
    }

    fn slot(self) -> usize {
        self.0 & Self::SLOT_MASK
    }

    fn generation(self) -> usize {
        self.0 >> Self::SLOT_BITS
    }

    /// Opaque generation-checked native identity, not an arena subscript.
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentTypeData {
    pub name: Option<String>,
    pub public_identifier: Option<String>,
    pub system_identifier: Option<String>,
    pub force_quirks: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DocumentMode {
    #[default]
    NoQuirks,
    LimitedQuirks,
    Quirks,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Document,
    Element(ElementData),
    Text(String),
    Comment(String),
    DocumentType(DocumentTypeData),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentError {
    UnknownParent,
    UnknownChild,
    SelfParenting,
    AncestorCycle,
    InvalidReference,
    NotAChild,
    InvalidGcRoot,
}

#[derive(Debug)]
struct NodeSlot {
    node: Option<Node>,
    generation: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomGcTelemetry {
    pub successful: usize,
    pub failed: usize,
    pub reclaimed: usize,
    pub live_nodes: usize,
    pub reserved_slots: usize,
    pub last_pause: Option<Duration>,
    pub max_pause: Duration,
}

#[derive(Debug)]
pub struct Document {
    nodes: Vec<NodeSlot>,
    free_nodes: Vec<usize>,
    live_nodes: usize,
    gc_runs: usize,
    gc_failures: usize,
    gc_reclaimed: usize,
    last_gc_pause: Option<Duration>,
    max_gc_pause: Duration,
    root: NodeId,
    mode: DocumentMode,
}

impl Document {
    pub fn new() -> Self {
        Self {
            nodes: vec![NodeSlot {
                node: Some(Node {
                    kind: NodeKind::Document,
                    parent: None,
                    children: Vec::new(),
                }),
                generation: 0,
            }],
            free_nodes: Vec::new(),
            live_nodes: 1,
            gc_runs: 0,
            gc_failures: 0,
            gc_reclaimed: 0,
            last_gc_pause: None,
            max_gc_pause: Duration::ZERO,
            root: NodeId(0),
            mode: DocumentMode::NoQuirks,
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn mode(&self) -> DocumentMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: DocumentMode) {
        self.mode = mode;
    }

    pub fn create_element(&mut self, tag_name: impl Into<String>) -> NodeId {
        self.create_element_with_attributes(tag_name, Vec::new())
    }

    pub fn create_element_with_attributes(
        &mut self,
        tag_name: impl Into<String>,
        attributes: Vec<Attribute>,
    ) -> NodeId {
        self.push_node(NodeKind::Element(ElementData {
            tag_name: tag_name.into(),
            attributes,
        }))
    }

    pub fn create_text(&mut self, text: impl Into<String>) -> NodeId {
        self.push_node(NodeKind::Text(text.into()))
    }

    pub fn create_comment(&mut self, data: impl Into<String>) -> NodeId {
        self.push_node(NodeKind::Comment(data.into()))
    }

    pub fn create_document_type(&mut self, data: DocumentTypeData) -> NodeId {
        self.push_node(NodeKind::DocumentType(data))
    }

    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), DocumentError> {
        self.insert_before(parent, child, None)
    }

    pub fn insert_before(
        &mut self,
        parent: NodeId,
        child: NodeId,
        reference: Option<NodeId>,
    ) -> Result<(), DocumentError> {
        if self.node(parent).is_none() {
            return Err(DocumentError::UnknownParent);
        }
        if self.node(child).is_none() {
            return Err(DocumentError::UnknownChild);
        }
        if let Some(reference) = reference
            && self.node(reference).is_none()
        {
            return Err(DocumentError::UnknownChild);
        }
        if parent == child {
            return Err(DocumentError::SelfParenting);
        }
        if let Some(reference) = reference {
            if self.node(reference).unwrap().parent != Some(parent) {
                return Err(DocumentError::InvalidReference);
            }
            if reference == child {
                return Ok(());
            }
        }
        // Never allow a descendant to acquire one of its ancestors,
        // including moves of detached subtrees. Check before detaching
        // the old parent so failures leave the document intact.
        let mut cursor = Some(parent);
        let mut remaining = self.nodes.len();
        while let Some(current) = cursor {
            if current == child {
                return Err(DocumentError::AncestorCycle);
            }
            if remaining == 0 {
                return Err(DocumentError::AncestorCycle);
            }
            remaining -= 1;
            cursor = self.node(current).unwrap().parent;
        }

        if let Some(old_parent) = self.node(child).unwrap().parent {
            self.node_mut(old_parent)
                .unwrap()
                .children
                .retain(|existing| *existing != child);
        }

        let insertion_index = reference.and_then(|reference| {
            self.node_mut(parent)
                .unwrap()
                .children
                .iter()
                .position(|existing| *existing == reference)
        });

        self.node_mut(child).unwrap().parent = Some(parent);
        match insertion_index {
            Some(index) => self.node_mut(parent).unwrap().children.insert(index, child),
            None => self.node_mut(parent).unwrap().children.push(child),
        }
        Ok(())
    }

    /// Replace an existing direct child without losing node IDs or partial
    /// mutations on failure. Reusing the old child is a no-op.
    pub fn replace_child(
        &mut self,
        parent: NodeId,
        new_child: NodeId,
        old_child: NodeId,
    ) -> Result<(), DocumentError> {
        if self.node(parent).is_none() {
            return Err(DocumentError::UnknownParent);
        }
        if self.node(old_child).is_none() || self.node(new_child).is_none() {
            return Err(DocumentError::UnknownChild);
        }
        if self.node(old_child).unwrap().parent != Some(parent) {
            return Err(DocumentError::NotAChild);
        }
        if old_child == new_child {
            return Ok(());
        }
        self.insert_before(parent, new_child, Some(old_child))?;
        self.remove_child(parent, old_child)?;
        Ok(())
    }

    /// Detach a direct child without deleting its identity or descendants.
    /// Invalid removals leave both nodes and their linkage unchanged.
    pub fn remove_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), DocumentError> {
        if self.node(parent).is_none() {
            return Err(DocumentError::UnknownParent);
        }
        if self.node(child).is_none() {
            return Err(DocumentError::UnknownChild);
        }
        if self.node(child).unwrap().parent != Some(parent) {
            return Err(DocumentError::NotAChild);
        }
        self.node_mut(parent)
            .unwrap()
            .children
            .retain(|&id| id != child);
        self.node_mut(child).unwrap().parent = None;
        Ok(())
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        let slot = self.nodes.get(id.slot())?;
        (slot.generation == id.generation())
            .then_some(slot.node.as_ref())
            .flatten()
    }

    /// Validate an opaque native handle received from the page VM or UI.
    pub fn node_id(&self, index: usize) -> Option<NodeId> {
        let id = NodeId(index);
        self.node(id).map(|_| id)
    }

    fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        let slot = self.nodes.get_mut(id.slot())?;
        if slot.generation != id.generation() {
            return None;
        }
        slot.node.as_mut()
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.node(id)
            .map(|node| node.children.as_slice())
            .unwrap_or_default()
    }

    pub fn element(&self, id: NodeId) -> Option<&ElementData> {
        match &self.node(id)?.kind {
            NodeKind::Element(element) => Some(element),
            _ => None,
        }
    }

    pub fn element_mut(&mut self, id: NodeId) -> Option<&mut ElementData> {
        match &mut self.node_mut(id)?.kind {
            NodeKind::Element(element) => Some(element),
            _ => None,
        }
    }

    /// Replace an element's child subtree with one ordinary text node.
    /// Detached descendants stay addressable by NodeId but are no longer
    /// reachable from the document root or considered during style/layout.
    pub fn set_text_content(&mut self, element: NodeId, text: &str) -> bool {
        if self.element(element).is_none() || text.len() > 64 * 1024 {
            return false;
        }
        let old_children = std::mem::take(&mut self.node_mut(element).unwrap().children);
        for old in old_children {
            self.node_mut(old).unwrap().parent = None;
        }
        if !text.is_empty() {
            let replacement = self.create_text(text);
            self.append_child(element, replacement)
                .expect("new text node and parent are valid");
        }
        true
    }

    pub fn set_text_node_content(&mut self, node: NodeId, text: &str) -> bool {
        if text.len() > 64 * 1024 {
            return false;
        }
        let Some(node) = self.node_mut(node) else {
            return false;
        };
        match &mut node.kind {
            NodeKind::Text(value) => {
                *value = text.to_owned();
                true
            }
            _ => false,
        }
    }

    pub fn document_type(&self, id: NodeId) -> Option<&DocumentTypeData> {
        match &self.node(id)?.kind {
            NodeKind::DocumentType(data) => Some(data),
            _ => None,
        }
    }

    /// Live native nodes, including retained detached trees.
    pub fn len(&self) -> usize {
        self.live_nodes
    }

    pub fn is_empty(&self) -> bool {
        self.live_nodes == 0
    }

    pub fn reserved_slots(&self) -> usize {
        self.nodes.len()
    }

    pub fn allocation_headroom(&self, slot_budget: usize) -> usize {
        self.free_nodes
            .len()
            .saturating_add(slot_budget.saturating_sub(self.nodes.len()))
    }

    pub fn gc_telemetry(&self) -> DomGcTelemetry {
        DomGcTelemetry {
            successful: self.gc_runs,
            failed: self.gc_failures,
            reclaimed: self.gc_reclaimed,
            live_nodes: self.live_nodes,
            reserved_slots: self.nodes.len(),
            last_pause: self.last_gc_pause,
            max_pause: self.max_gc_pause,
        }
    }

    /// Idle-only host sweep. The document tree and every retained root's whole
    /// native tree component survive. Validate all handles before any mutation.
    /// Call only after parsing and validated JS GC, with no pending host writes.
    pub fn collect_garbage(&mut self, host_roots: &[usize]) -> Result<Vec<NodeId>, DocumentError> {
        let started = Instant::now();
        let mut marked = vec![false; self.nodes.len()];
        let mut pending = vec![self.root];
        for &handle in host_roots {
            let Some(id) = self.node_id(handle) else {
                self.gc_failures = self.gc_failures.saturating_add(1);
                return Err(DocumentError::InvalidGcRoot);
            };
            pending.push(id);
        }
        while let Some(id) = pending.pop() {
            let Some(node) = self.node(id) else {
                self.gc_failures = self.gc_failures.saturating_add(1);
                return Err(DocumentError::InvalidGcRoot);
            };
            if marked[id.slot()] {
                continue;
            }
            marked[id.slot()] = true;
            pending.extend(node.parent);
            pending.extend(node.children.iter().copied());
        }
        let mut freed = Vec::new();
        for (index, slot) in self.nodes.iter_mut().enumerate() {
            if slot.node.is_none() || marked[index] {
                continue;
            }
            freed.push(NodeId::from_slot(index, slot.generation));
            slot.node = None; // Drop text, attributes and child vectors.
            if slot.generation < NodeId::MAX_GENERATION {
                slot.generation += 1;
                self.free_nodes.push(index);
            }
        }
        self.live_nodes -= freed.len();
        self.gc_runs = self.gc_runs.saturating_add(1);
        self.gc_reclaimed = self.gc_reclaimed.saturating_add(freed.len());
        let pause = started.elapsed();
        self.last_gc_pause = Some(pause);
        self.max_gc_pause = self.max_gc_pause.max(pause);
        Ok(freed)
    }

    fn push_node(&mut self, kind: NodeKind) -> NodeId {
        let node = Node {
            kind,
            parent: None,
            children: Vec::new(),
        };
        self.live_nodes += 1;
        if let Some(index) = self.free_nodes.pop() {
            let slot = &mut self.nodes[index];
            slot.node = Some(node);
            NodeId::from_slot(index, slot.generation)
        } else {
            let id = NodeId::from_slot(self.nodes.len(), 0);
            self.nodes.push(NodeSlot {
                node: Some(node),
                generation: 0,
            });
            id
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod dynamic_mutation_tests {
    use super::*;
    #[test]
    fn reject_ancestor_cycles_without_damaging_existing_subtree() {
        let mut document = Document::new();
        let parent = document.create_element("div");
        let child = document.create_element("p");
        let grandchild = document.create_element("span");
        document.append_child(document.root(), parent).unwrap();
        document.append_child(parent, child).unwrap();
        document.append_child(child, grandchild).unwrap();
        assert_eq!(
            document.append_child(grandchild, parent),
            Err(DocumentError::AncestorCycle)
        );
        assert_eq!(
            document.append_child(child, child),
            Err(DocumentError::SelfParenting)
        );
        assert_eq!(document.node(parent).unwrap().parent, Some(document.root()));
        assert_eq!(document.children(parent), &[child]);
        assert_eq!(document.node(grandchild).unwrap().parent, Some(child));
    }
}

#[cfg(test)]
mod dynamic_remove_move_tests {
    use super::*;
    #[test]
    fn remove_and_reinsert_keep_node_identity_and_protect_invalid_reference() {
        let mut dom = Document::new();
        let root = dom.create_element("main");
        let first = dom.create_element("p");
        let second = dom.create_element("b");
        let foreign = dom.create_element("i");
        dom.append_child(dom.root(), root).unwrap();
        dom.append_child(root, first).unwrap();
        dom.append_child(root, second).unwrap();
        assert_eq!(
            dom.insert_before(root, second, Some(foreign)),
            Err(DocumentError::InvalidReference)
        );
        assert_eq!(dom.children(root), &[first, second]);
        dom.insert_before(root, second, Some(first)).unwrap();
        assert_eq!(dom.children(root), &[second, first]);
        assert_eq!(dom.remove_child(root, first), Ok(()));
        assert_eq!(dom.remove_child(root, first), Err(DocumentError::NotAChild));
        assert_eq!(dom.node(first).unwrap().parent, None);
        dom.append_child(root, first).unwrap();
        assert_eq!(dom.children(root), &[second, first]);
        assert_eq!(dom.node(first).unwrap().parent, Some(root));
    }
}

#[cfg(test)]
mod replace_child_tests {
    use super::*;
    #[test]
    fn replace_child_is_ordered_and_rejects_cross_parent_reference() {
        let mut dom = Document::new();
        let a = dom.create_element("main");
        let b = dom.create_element("section");
        let first = dom.create_element("p");
        let old = dom.create_element("em");
        let last = dom.create_element("strong");
        let next = dom.create_element("i");
        dom.append_child(dom.root(), a).unwrap();
        dom.append_child(dom.root(), b).unwrap();
        for child in [first, old, last] {
            dom.append_child(a, child).unwrap();
        }
        dom.append_child(b, next).unwrap();
        assert_eq!(
            dom.replace_child(b, first, old),
            Err(DocumentError::NotAChild)
        );
        assert_eq!(dom.children(a), &[first, old, last]);
        dom.replace_child(a, next, old).unwrap();
        assert_eq!(dom.children(a), &[first, next, last]);
        assert_eq!(dom.node(old).unwrap().parent, None);
        assert_eq!(dom.node(next).unwrap().parent, Some(a));
        dom.replace_child(a, next, next).unwrap();
        assert_eq!(dom.children(a), &[first, next, last]);
        assert_eq!(
            dom.replace_child(next, a, first),
            Err(DocumentError::NotAChild)
        );
    }
}

#[cfg(test)]
mod gc_tests {
    use super::*;

    #[test]
    fn roots_preserve_detached_ancestors_siblings_and_unwrapped_descendants() {
        let mut dom = Document::new();
        let parent = dom.create_element("section");
        let child = dom.create_element("span");
        let sibling = dom.create_text("sibling");
        let text = dom.create_text("child text");
        dom.append_child(parent, child).unwrap();
        dom.append_child(parent, sibling).unwrap();
        dom.append_child(child, text).unwrap();
        assert!(dom.collect_garbage(&[child.index()]).unwrap().is_empty());
        assert_eq!(dom.node(child).unwrap().parent, Some(parent));
        assert_eq!(dom.children(parent), &[child, sibling]);
        assert_eq!(dom.children(child), &[text]);
        assert_eq!(dom.len(), 5);
        assert_eq!(dom.collect_garbage(&[]).unwrap().len(), 4);
        assert_eq!(dom.len(), 1);
        assert_eq!(dom.node_id(child.index()), None);
    }

    #[test]
    fn repeated_sweeps_recycle_slots_and_reject_every_stale_mutation() {
        let mut dom = Document::new();
        let old = dom.create_element("old");
        dom.collect_garbage(&[]).unwrap();
        let new = dom.create_element("new");
        assert_eq!(old.slot(), new.slot());
        assert_ne!(old, new);
        assert!(new.index() < usize::MAX / 2);
        assert_eq!(dom.node_id(old.index()), None);
        assert_eq!(
            dom.append_child(old, new),
            Err(DocumentError::UnknownParent)
        );
        assert_eq!(dom.append_child(new, old), Err(DocumentError::UnknownChild));
        assert_eq!(
            dom.remove_child(dom.root(), old),
            Err(DocumentError::UnknownChild)
        );
        assert!(!dom.set_text_content(old, "corrupt"));
        assert!(!dom.set_text_node_content(old, "corrupt"));
        assert!(dom.element_mut(old).is_none());
        dom.collect_garbage(&[]).unwrap();
        for _ in 0..500 {
            let a = dom.create_element("a");
            let b = dom.create_text("owned native string");
            dom.append_child(a, b).unwrap();
            assert_eq!(dom.collect_garbage(&[]).unwrap().len(), 2);
            assert_eq!(dom.len(), 1);
            assert_eq!(dom.reserved_slots(), 3);
            assert!(dom.node(a).is_none());
        }
    }

    #[test]
    fn invalid_root_aborts_before_freeing_any_other_node() {
        let mut dom = Document::new();
        let dead = dom.create_text("dead");
        dom.collect_garbage(&[]).unwrap();
        let kept = dom.create_text("still addressable");
        let before = dom.gc_telemetry();
        assert_eq!(
            dom.collect_garbage(&[dead.index()]),
            Err(DocumentError::InvalidGcRoot)
        );
        assert!(dom.node(kept).is_some());
        let after = dom.gc_telemetry();
        assert_eq!(after.successful, before.successful);
        assert_eq!(after.reclaimed, before.reclaimed);
        assert_eq!(after.failed, before.failed + 1);
        assert_eq!(after.last_pause, before.last_pause);
    }

    #[test]
    fn exhausted_generation_retires_a_slot_instead_of_aliasing_old_handles() {
        let mut dom = Document::new();
        let old = dom.create_text("retired");
        dom.nodes[old.slot()].generation = NodeId::MAX_GENERATION;
        let retired = NodeId::from_slot(old.slot(), NodeId::MAX_GENERATION);
        assert_eq!(dom.collect_garbage(&[]).unwrap(), vec![retired]);
        let new = dom.create_text("new slot");
        assert_ne!(retired.slot(), new.slot());
        assert!(dom.node_id(retired.index()).is_none());
        assert!(dom.node_id(old.index()).is_none());
    }
}
