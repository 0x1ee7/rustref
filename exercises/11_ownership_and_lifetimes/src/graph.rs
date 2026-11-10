//! Part 5b — an index-based (arena) dependency graph with a cached topological order.

/// Index of a node in one specific [`Graph`]. Ids from other graphs are not detected
/// unless they are out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    // engineer: usize index
}

/// Errors from [`Graph`] operations.
#[derive(Debug, PartialEq, Eq)]
pub enum GraphError {
    /// The id is not a node of this graph.
    UnknownNode(NodeId),
    /// A dependency cycle. `path` starts and ends with the same node; see README for ordering.
    Cycle { path: Vec<NodeId> },
}

/// Build-target dependency graph. Edges point from dependent to dependency.
#[derive(Debug, Default)]
pub struct Graph {
    // engineer: names, deps, OnceCell<Vec<NodeId>> topo cache, Cell<u32> computation counter
}

impl Graph {
    pub fn new() -> Self {
        todo!()
    }

    pub fn add_node(&mut self, _name: impl Into<String>) -> NodeId {
        todo!()
    }

    pub fn name(&self, _id: NodeId) -> Option<&str> {
        todo!()
    }

    /// `dependent` depends on `dependency`. Duplicate edges are ignored.
    pub fn add_edge(&mut self, _dependent: NodeId, _dependency: NodeId) -> Result<(), GraphError> {
        todo!()
    }

    /// Direct dependencies in insertion order. `None` for unknown ids.
    pub fn dependencies(&self, _id: NodeId) -> Option<&[NodeId]> {
        todo!()
    }

    /// Nodes that directly depend on `id`, in ascending id order.
    pub fn dependents(&self, _id: NodeId) -> Vec<NodeId> {
        todo!()
    }

    /// Dependencies-first order, cached until the graph changes.
    pub fn topo_order(&self) -> Result<&[NodeId], GraphError> {
        todo!()
    }

    /// How many times the topological order has actually been computed.
    pub fn topo_computations(&self) -> u32 {
        todo!()
    }
}
