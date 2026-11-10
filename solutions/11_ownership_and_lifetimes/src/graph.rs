//! Part 5b — an index-based (arena) dependency graph with a cached topological order.

use std::cell::{Cell, OnceCell};

/// Index of a node in one specific [`Graph`]. Ids from other graphs are not detected
/// unless they are out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    index: usize,
}

/// Errors from [`Graph`] operations.
#[derive(Debug, PartialEq, Eq)]
pub enum GraphError {
    /// The id is not a node of this graph.
    UnknownNode(NodeId),
    /// A dependency cycle. `path` starts and ends with the same node.
    Cycle { path: Vec<NodeId> },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Unvisited,
    OnStack,
    Done,
}

/// Build-target dependency graph. Edges point from dependent to dependency.
#[derive(Debug, Default)]
pub struct Graph {
    names: Vec<String>,
    deps: Vec<Vec<NodeId>>,
    topo: OnceCell<Vec<NodeId>>,
    computations: Cell<u32>,
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, name: impl Into<String>) -> NodeId {
        self.topo = OnceCell::new();
        self.names.push(name.into());
        self.deps.push(Vec::new());
        NodeId {
            index: self.names.len() - 1,
        }
    }

    pub fn name(&self, id: NodeId) -> Option<&str> {
        self.names.get(id.index).map(String::as_str)
    }

    /// `dependent` depends on `dependency`. Duplicate edges are ignored.
    pub fn add_edge(&mut self, dependent: NodeId, dependency: NodeId) -> Result<(), GraphError> {
        self.check(dependent)?;
        self.check(dependency)?;
        let deps = &mut self.deps[dependent.index];
        if !deps.contains(&dependency) {
            deps.push(dependency);
            self.topo = OnceCell::new();
        }
        Ok(())
    }

    /// Direct dependencies in insertion order. `None` for unknown ids.
    pub fn dependencies(&self, id: NodeId) -> Option<&[NodeId]> {
        self.deps.get(id.index).map(Vec::as_slice)
    }

    /// Nodes that directly depend on `id`, in ascending id order.
    pub fn dependents(&self, id: NodeId) -> Vec<NodeId> {
        (0..self.deps.len())
            .filter(|&i| self.deps[i].contains(&id))
            .map(|index| NodeId { index })
            .collect()
    }

    /// Dependencies-first order, cached until the graph changes.
    pub fn topo_order(&self) -> Result<&[NodeId], GraphError> {
        if let Some(order) = self.topo.get() {
            return Ok(order);
        }
        let order = self.compute_topo()?;
        self.computations.set(self.computations.get() + 1);
        Ok(self.topo.get_or_init(|| order))
    }

    /// How many times the topological order has actually been computed.
    pub fn topo_computations(&self) -> u32 {
        self.computations.get()
    }

    fn check(&self, id: NodeId) -> Result<(), GraphError> {
        if id.index < self.names.len() {
            Ok(())
        } else {
            Err(GraphError::UnknownNode(id))
        }
    }

    /// Iterative DFS post-order. Each stack frame is `(node, next dependency index)`.
    fn compute_topo(&self) -> Result<Vec<NodeId>, GraphError> {
        let n = self.names.len();
        let mut marks = vec![Mark::Unvisited; n];
        let mut order = Vec::with_capacity(n);

        for root in 0..n {
            if marks[root] != Mark::Unvisited {
                continue;
            }
            marks[root] = Mark::OnStack;
            let mut stack: Vec<(usize, usize)> = vec![(root, 0)];

            while let Some(&(node, next)) = stack.last() {
                match self.deps[node].get(next) {
                    Some(&dep) => {
                        if let Some(top) = stack.last_mut() {
                            top.1 += 1;
                        }
                        match marks[dep.index] {
                            Mark::Unvisited => {
                                marks[dep.index] = Mark::OnStack;
                                stack.push((dep.index, 0));
                            }
                            Mark::OnStack => {
                                let start = stack
                                    .iter()
                                    .position(|&(n, _)| n == dep.index)
                                    .expect("an on-stack node is on the stack");
                                let mut path: Vec<NodeId> = stack[start..]
                                    .iter()
                                    .map(|&(index, _)| NodeId { index })
                                    .collect();
                                path.push(dep);
                                return Err(GraphError::Cycle { path });
                            }
                            Mark::Done => {}
                        }
                    }
                    None => {
                        stack.pop();
                        marks[node] = Mark::Done;
                        order.push(NodeId { index: node });
                    }
                }
            }
        }
        Ok(order)
    }
}
