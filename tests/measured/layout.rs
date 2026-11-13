//! What a reader pays to stand every thing of a publication on the map, by
//! the millisecond rule: every focus of a cluster-sized publication laid out
//! within a millisecond (`topology::layout::every_focus`, which the
//! runtime's `xmip_publication_layout_v1` computes once per publication it
//! reads; ADR-0052, amendment 2026-11-13). On 2026-11-13 the first build of
//! it cost a .NET reader about 100 ms a read of the 176-node publication
//! `SnapshotReadCostTest` reads: every place of the operation band searched
//! by copying the map a hundred times, and every id found by a scan.

use std::time::{Duration, Instant};

use node::{Capability, NodeRole};
use xmip_core_observe::topology::draw::members;
use xmip_core_observe::topology::layout::{every_focus, place_order};
use xmip_core_observe::topology::party;
use xmip_core_observe::topology::relate::{Member, relations};
use xmip_core_observe::{Health, HealthRecord, NodeKind, Snapshot, Topology};

/// How many things a cluster-sized publication draws, as
/// `SnapshotReadCostTest` draws them.
const DRAWN: usize = 176;

/// Every place of every focus `topology` has, each laid out once, as a
/// reader asks as it reads a publication.
fn stood(topology: &Topology) -> usize {
    every_focus(topology).values().map(Vec::len).sum()
}

/// The fastest of five of fifty layouts of every focus of `topology`, held
/// to the millisecond.
fn timed(what: &str, topology: &Topology, placed: usize) {
    let mut taken = Vec::new();
    for _ in 0..50 {
        let started = Instant::now();
        let count = stood(topology);
        taken.push(started.elapsed());
        assert_eq!(count, placed, "{what}");
    }
    let spread = xcore::latency::spread(what, taken);
    assert!(
        spread.fastest_of_five < Duration::from_millis(1),
        "{spread:?}"
    );
}

/// The publication `SnapshotReadCostTest` reads, as far as its drawing goes:
/// the cluster and 175 nodes beneath it, none declaring a role, no line.
#[test]
fn every_focus_of_a_cluster_sized_publication_is_laid_out_within_a_millisecond() {
    let cluster = configure::fixture::test_cluster();
    let root = cluster.scope();
    let named: Vec<Member> = (1..DRAWN)
        .map(|at| Member::new(&format!("{}-{at}", cluster.node(at % 9).name), &[], None))
        .collect();
    let topology = Topology {
        nodes: members(&Snapshot::new(), &root, &named),
        ..Topology::default()
    };
    assert_eq!(topology.nodes.len(), DRAWN);
    timed(
        "every focus of 176 drawn things, none declaring a role",
        &topology,
        DRAWN,
    );
}

/// A cluster related by role drawing about as many things — an operational
/// and monitoring node relating to every other, two Storage nodes listed,
/// the rest receiving, sending, processing and executing, each node's
/// stages drawn and a Party facing each — so the operation band's lines fan
/// to every node and its place is searched.
#[test]
fn a_cluster_related_by_role_drawing_as_many_things_is_laid_out_within_a_millisecond() {
    let cluster = configure::fixture::test_cluster();
    let root = cluster.scope();
    let roles = [
        NodeRole::Receiving,
        NodeRole::Sending,
        NodeRole::Processing,
        NodeRole::Executing,
    ];
    let mut named = vec![
        Member::new(
            "operating",
            &[NodeRole::Operational, NodeRole::Monitoring],
            None,
        ),
        Member::new("storing-0", &[NodeRole::Storage], Some(0)),
        Member::new("storing-1", &[NodeRole::Storage], Some(1)),
    ];
    named.extend((3..52).map(|at| {
        let base = &cluster.node(at % 9).name;
        Member::new(&format!("{base}-{at}"), &[roles[at % 4]], None)
    }));
    let mut snapshot = Snapshot::new();
    for member in &named {
        snapshot.record_health(HealthRecord {
            scope: format!("{root}/node/{}/capability", member.node),
            health: Health::Fine,
            severity: 0,
            evidence: Capability::of(&member.roles).evidence(),
            observed_unix_nanos: 7,
        });
    }
    let mut topology = Topology {
        nodes: members(&snapshot, &root, &named),
        links: relations(&root, &named, &snapshot),
        ..Topology::default()
    };
    let ordered: Vec<&str> = place_order(&named)
        .into_iter()
        .map(|member| member.node.as_str())
        .collect();
    party::facing(&snapshot, &root, &ordered, &[], &mut topology);
    assert!(topology.nodes.len() >= DRAWN, "{}", topology.nodes.len());
    let placed = topology
        .nodes
        .iter()
        .filter(|node| node.kind != NodeKind::Cluster)
        .count()
        + 1;
    timed(
        "every focus of a cluster related by role drawing 176 things",
        &topology,
        placed,
    );
}
