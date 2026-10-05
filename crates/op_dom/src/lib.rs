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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Document,
    Element(ElementData),
    Text(String),
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
}

#[derive(Debug)]
pub struct Document {
    nodes: Vec<Node>,
    root: NodeId,
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
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
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

    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> Result<(), DocumentError> {
        if parent.index() >= self.nodes.len() {
            return Err(DocumentError::UnknownParent);
        }
        if child.index() >= self.nodes.len() {
            return Err(DocumentError::UnknownChild);
        }
        if parent == child {
            return Err(DocumentError::SelfParenting);
        }

        if let Some(old_parent) = self.nodes[child.index()].parent {
            self.nodes[old_parent.index()]
                .children
                .retain(|existing| *existing != child);
        }

        self.nodes[child.index()].parent = Some(parent);
        self.nodes[parent.index()].children.push(child);
        Ok(())
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.index())
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
