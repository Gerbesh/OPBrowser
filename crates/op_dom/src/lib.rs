#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

impl NodeId {
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
}

#[derive(Debug)]
pub struct Document {
    nodes: Vec<Node>,
    root: NodeId,
    mode: DocumentMode,
}

impl Document {
    pub fn new() -> Self {
        Self {
            nodes: vec![Node {
                kind: NodeKind::Document,
                parent: None,
                children: Vec::new(),
            }],
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
        if parent.index() >= self.nodes.len() {
            return Err(DocumentError::UnknownParent);
        }
        if child.index() >= self.nodes.len() {
            return Err(DocumentError::UnknownChild);
        }
        if let Some(reference) = reference
            && reference.index() >= self.nodes.len()
        {
            return Err(DocumentError::UnknownChild);
        }
        if parent == child {
            return Err(DocumentError::SelfParenting);
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
            cursor = self.nodes[current.index()].parent;
        }

        if let Some(old_parent) = self.nodes[child.index()].parent {
            self.nodes[old_parent.index()]
                .children
                .retain(|existing| *existing != child);
        }

        let insertion_index = reference.and_then(|reference| {
            self.nodes[parent.index()]
                .children
                .iter()
                .position(|existing| *existing == reference)
        });

        self.nodes[child.index()].parent = Some(parent);
        match insertion_index {
            Some(index) => self.nodes[parent.index()].children.insert(index, child),
            None => self.nodes[parent.index()].children.push(child),
        }
        Ok(())
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.index())
    }

    pub fn node_id(&self, index: usize) -> Option<NodeId> {
        (index < self.nodes.len()).then_some(NodeId(index))
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
        match &mut self.nodes.get_mut(id.index())?.kind {
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
        let old_children = std::mem::take(&mut self.nodes[element.index()].children);
        for old in old_children {
            self.nodes[old.index()].parent = None;
        }
        if !text.is_empty() {
            let replacement = self.create_text(text);
            self.append_child(element, replacement)
                .expect("new text node and parent are valid");
        }
        true
    }

    pub fn document_type(&self, id: NodeId) -> Option<&DocumentTypeData> {
        match &self.node(id)?.kind {
            NodeKind::DocumentType(data) => Some(data),
            _ => None,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    fn push_node(&mut self, kind: NodeKind) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(Node {
            kind,
            parent: None,
            children: Vec::new(),
        });
        id
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
