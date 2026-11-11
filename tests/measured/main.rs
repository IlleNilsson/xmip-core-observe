//! The module's timing program (`doc/governance/rust-style.md` section
//! 5c): what a publisher pays each round to state what moved on its
//! drawing, held to the millisecond rule. A plain `cargo test` leaves it
//! out; the landing runs it after every other test of the module, alone,
//! one test at a time (`cargo test --test measured --all-features --
//! --test-threads=1`), so what is measured is Xmip's work and not its
//! neighbors':
//!
//! - A window states a drawing of 1,201 things, 3,000 counts and 1,000
//!   links within a millisecond (`topology::window`; ADR-0052, amendment
//!   2026-11-10): one pass a round, never the square of the cluster, which
//!   took 101 ms for as many as first built.

use std::time::{Duration, Instant};

use xmip_core_observe::topology::window::Window;
use xmip_core_observe::{
    Count, Counted, Health, NodeKind, Origin, Pattern, Snapshot, Topology, TopologyLink,
    TopologyNode,
};

/// How many nodes the drawing has: each drawn with its three stages and a
/// Location beneath its receive and its send stage, six things a node.
const NODES: usize = 200;

/// A quarter of a second, in nanoseconds: how often `xmip-service`
/// publishes.
const QUARTER: i64 = 250_000_000;

fn thing(id: String, parent: &str, kind: NodeKind, scope: String) -> TopologyNode {
    TopologyNode {
        id,
        parent: parent.to_string(),
        label: String::new(),
        kind,
        scope,
        state: Health::Fine,
        origin: Origin::Both,
        load: 0.0,
        activity: 0.0,
        evidence: String::new(),
    }
}

fn link(id: String, from: String, to: String) -> TopologyLink {
    TopologyLink {
        id,
        from,
        to,
        pattern: Pattern::SendReceive,
        origin: Origin::Both,
        protocol: String::new(),
        state: Health::Fine,
        volume: 0,
        rate: 0.0,
        latency_ms: 0.0,
        progress: 0.0,
        attempts: 0,
        evidence: String::new(),
    }
}

fn count(scope: String, counted: Counted, value: u64) -> Count {
    Count {
        scope,
        counted,
        value,
        window_start_unix_nanos: 0,
        window_end_unix_nanos: 0,
        observed_unix_nanos: 0,
    }
}

/// A cluster of [`NODES`] nodes as a publisher draws it — 1,201 things —
/// with fifteen counts a node beneath its Locations, and five links a node:
/// a Party sending in, one delivered to, receive to process, process to
/// send, and a handoff to the next node's send stage.
fn drawing() -> (Topology, Snapshot) {
    let cluster = configure::fixture::test_cluster();
    let root = cluster.scope();
    let name = cluster.node(0).name.clone();
    let mut nodes = vec![thing(
        "cluster".to_string(),
        "",
        NodeKind::Cluster,
        root.clone(),
    )];
    let mut links = Vec::new();
    let mut snapshot = Snapshot::new();
    let (receives, sends) = ("receive/Orders from Party-A", "send/Invoices to Party-B");
    for n in 0..NODES {
        let (id, scope) = (format!("node/{name}{n}"), format!("{root}/node/{name}{n}"));
        nodes.push(thing(id.clone(), "cluster", NodeKind::Node, scope.clone()));
        for stage in ["receive", "process", "send"] {
            let (at, within) = (format!("{id}/{stage}"), format!("{scope}/{stage}"));
            nodes.push(thing(at, &id, NodeKind::Stage, within));
        }
        for leaf in [receives, sends] {
            let (stage, _) = leaf.split_once('/').expect("a stage and a Location");
            let (at, within) = (format!("{id}/{leaf}"), format!("{scope}/{leaf}"));
            nodes.push(thing(
                at,
                &format!("{id}/{stage}"),
                NodeKind::Endpoint,
                within,
            ));
        }
        for part in 0..5 {
            for (leaf, counted) in [
                (receives, Counted::Streams),
                (receives, Counted::Bytes),
                (sends, Counted::Messages),
            ] {
                let value = u64::try_from(n * part).expect("small");
                snapshot.record_count(count(format!("{scope}/{leaf}/{part}"), counted, value));
            }
        }
        let next = format!("node/{name}{}/send", (n + 1) % NODES);
        for (from, to) in [
            ("party/sending/party-a".to_string(), format!("{id}/receive")),
            (format!("{id}/send"), "party/receiving/party-b".to_string()),
            (format!("{id}/receive"), format!("{id}/process")),
            (format!("{id}/process"), format!("{id}/send")),
            (format!("{id}/process"), next),
        ] {
            links.push(link(format!("{from}/{to}"), from, to));
        }
    }
    let topology = Topology {
        source: "measured".to_string(),
        observed_unix_nanos: 0,
        nodes,
        links,
    };
    (topology, snapshot)
}

#[test]
fn a_window_states_a_drawing_of_twelve_hundred_things_within_a_millisecond() {
    let (drawn, snapshot) = drawing();
    assert_eq!(
        (
            drawn.nodes.len(),
            snapshot.all_counts().count(),
            drawn.links.len()
        ),
        (1_201, 3_000, 1_000)
    );
    let mut window = Window::new();
    let mut taken = Vec::new();
    // Past the half minute first, so the window holds what it holds as it
    // runs: about thirty readings of each figure.
    for round in 0..200 {
        let mut topology = drawn.clone();
        topology.observed_unix_nanos = round * QUARTER;
        let carried = u64::try_from(round).expect("small");
        for link in &mut topology.links {
            link.volume = carried;
        }
        let started = Instant::now();
        window.state(&mut topology, &snapshot);
        let took = started.elapsed();
        if round >= 150 {
            taken.push(took);
            assert!(
                topology.links.iter().all(|link| link.rate > 0.0),
                "each moved"
            );
        }
    }
    let spread = xcore::latency::spread(
        "a window stating 1,201 drawn things, 3,000 counts and 1,000 links",
        taken,
    );
    assert!(
        spread.fastest_of_five < Duration::from_millis(1),
        "{spread:?}"
    );
}
