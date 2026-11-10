use ex11_ownership_and_lifetimes::graph::{Graph, GraphError};

/// Maps node ids to their names so tests can compare readable sequences.
fn names(g: &Graph, ids: &[ex11_ownership_and_lifetimes::graph::NodeId]) -> Vec<String> {
    ids.iter().map(|id| g.name(*id).unwrap().to_string()).collect()
}

#[test]
fn part5_topo_order_lists_dependencies_first() {
    // app → lib, app → util, lib → core, util → core
    let mut g = Graph::new();
    let app = g.add_node("app");
    let lib = g.add_node("lib");
    let util = g.add_node("util");
    let core = g.add_node("core");
    g.add_edge(app, lib).unwrap();
    g.add_edge(app, util).unwrap();
    g.add_edge(lib, core).unwrap();
    g.add_edge(util, core).unwrap();

    let order = g.topo_order().unwrap().to_vec();
    assert_eq!(names(&g, &order), vec!["core", "lib", "util", "app"]);
}

#[test]
fn part5_independent_nodes_keep_insertion_order() {
    let mut g = Graph::new();
    g.add_node("a");
    g.add_node("b");
    g.add_node("c");
    let order = g.topo_order().unwrap().to_vec();
    assert_eq!(names(&g, &order), vec!["a", "b", "c"]);
}

#[test]
fn part5_cycle_is_reported_with_closed_path() {
    let mut g = Graph::new();
    let a = g.add_node("a");
    let b = g.add_node("b");
    let c = g.add_node("c");
    g.add_edge(a, b).unwrap();
    g.add_edge(b, c).unwrap();
    g.add_edge(c, a).unwrap();
    match g.topo_order() {
        Err(GraphError::Cycle { path }) => {
            assert_eq!(names(&g, &path), vec!["a", "b", "c", "a"]);
        }
        other => panic!("expected a cycle, got {other:?}"),
    }
}

#[test]
fn part5_self_loop_is_a_cycle() {
    let mut g = Graph::new();
    let a = g.add_node("a");
    g.add_edge(a, a).unwrap();
    match g.topo_order() {
        Err(GraphError::Cycle { path }) => assert_eq!(names(&g, &path), vec!["a", "a"]),
        other => panic!("expected a cycle, got {other:?}"),
    }
}

#[test]
fn part5_edge_with_foreign_node_is_rejected() {
    let mut other = Graph::new();
    other.add_node("a");
    let b = other.add_node("b"); // index 1 does not exist in `g` below

    let mut g = Graph::new();
    let a = g.add_node("a");
    assert_eq!(g.add_edge(a, b), Err(GraphError::UnknownNode(b)));
    assert!(g.dependencies(a).unwrap().is_empty(), "rejected edge must not be stored");
}

#[test]
fn part5_duplicate_edges_are_ignored() {
    let mut g = Graph::new();
    let a = g.add_node("a");
    let b = g.add_node("b");
    g.add_edge(a, b).unwrap();
    g.add_edge(a, b).unwrap();
    assert_eq!(g.dependencies(a).unwrap().len(), 1);
}

#[test]
fn part5_dependents_are_reverse_edges_in_id_order() {
    let mut g = Graph::new();
    let app = g.add_node("app");
    let lib = g.add_node("lib");
    let util = g.add_node("util");
    let core = g.add_node("core");
    g.add_edge(app, lib).unwrap();
    g.add_edge(app, util).unwrap();
    g.add_edge(lib, core).unwrap();
    g.add_edge(util, core).unwrap();
    assert_eq!(names(&g, &g.dependents(core)), vec!["lib", "util"]);
    assert!(g.dependents(app).is_empty());
}

#[test]
fn part5_topo_order_is_cached_until_mutation() {
    let mut g = Graph::new();
    g.add_node("a");
    g.add_node("b");
    assert_eq!(g.topo_computations(), 0);
    g.topo_order().unwrap();
    assert_eq!(g.topo_computations(), 1);
    g.topo_order().unwrap();
    assert_eq!(g.topo_computations(), 1, "second call must reuse the cache");

    g.add_node("z");
    g.topo_order().unwrap();
    assert_eq!(g.topo_computations(), 2, "mutation must invalidate the cache");
}

#[test]
fn part5_cached_order_reflects_later_edges() {
    let mut g = Graph::new();
    let a = g.add_node("a");
    let b = g.add_node("b");
    assert_eq!(names(&g, g.topo_order().unwrap()), vec!["a", "b"]);
    g.add_edge(a, b).unwrap();
    assert_eq!(names(&g, g.topo_order().unwrap()), vec!["b", "a"], "stale cache would hide the edge");
}
