use super::{Party, RX_PARTY, TX_PARTY, draw, facing};
use crate::health::Health;
use crate::snapshot::{Count, HealthRecord, Snapshot};
use crate::topology::relate::Member;
use crate::topology::{Relation, Topology, draw::members, link_id};
use crate::{Counted, NodeKind};

use configure::fixture::test_cluster;

fn root() -> String {
    test_cluster().scope()
}

/// The test cluster's first node, the one drawn.
fn node() -> String {
    test_cluster().node(0).name.clone()
}

fn health_at(snapshot: &mut Snapshot, name: &str, leaf: &str, health: Health) {
    snapshot.record_health(HealthRecord {
        scope: format!("{}/node/{name}/{leaf}", root()),
        health,
        severity: if health == Health::Fine { 0 } else { 60 },
        evidence: format!("{leaf} said so"),
        observed_unix_nanos: 7,
    });
}

fn health(snapshot: &mut Snapshot, leaf: &str, health: Health) {
    health_at(snapshot, &node(), leaf, health);
}

fn count(snapshot: &mut Snapshot, at: &str, counted: Counted, value: u64) {
    snapshot.record_count(Count {
        scope: format!("{}/node/{}/{at}", root(), node()),
        counted,
        value,
        window_start_unix_nanos: 7,
        window_end_unix_nanos: 7,
        observed_unix_nanos: 7,
    });
}

/// The snapshot's nodes `names` drawn, declaring nothing.
fn drawing(snapshot: &Snapshot, names: &[&str]) -> Topology {
    let named: Vec<Member> = names
        .iter()
        .map(|name| Member::new(name, &[], None))
        .collect();
    Topology {
        source: String::new(),
        observed_unix_nanos: 9,
        nodes: members(snapshot, &root(), &named),
        links: Vec::new(),
    }
}

/// The test cluster's first node receiving over `a` and `b` and sending
/// over `a`, `b` failed at receive; two Parties, the first sending over `a`
/// and receiving over `b`, the second the other way round.
fn drawn() -> Topology {
    let mut snapshot = Snapshot::new();
    health(&mut snapshot, "receive/a/json", Health::Fine);
    health(&mut snapshot, "receive/b/json", Health::Done);
    health(&mut snapshot, "send/a/json", Health::Fine);
    count(&mut snapshot, "receive/a", Counted::Streams, 3);
    count(&mut snapshot, "receive/b", Counted::Streams, 4);
    count(&mut snapshot, "send/a", Counted::Messages, 5);
    let mut topology = drawing(&snapshot, &[node().as_str()]);
    let (a, b) = (["a".to_string()], ["b".to_string()]);
    let parties = [
        Party {
            name: "one",
            sends: Some(&a),
            receives: Some(&b),
        },
        Party {
            name: "two",
            sends: Some(&b),
            receives: Some(&a),
        },
    ];
    draw(
        &snapshot,
        &root(),
        &[node().as_str()],
        &parties,
        &mut topology,
    );
    topology
}

#[test]
fn each_party_is_drawn_over_its_own_transports_on_each_side() {
    let topology = drawn();
    let parties: Vec<&str> = topology
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Party)
        .map(|node| node.id.as_str())
        .collect();
    // The first is delivered to over b, which no send stage holds.
    assert_eq!(
        parties,
        [
            "party/sending/one",
            "party/sending/two",
            "party/receiving/two"
        ]
    );
    let link = |from: &str, to: &str| {
        let id = link_id(from, to, Relation::Traffic);
        topology
            .links
            .iter()
            .find(|link| link.id == id)
            .unwrap_or_else(|| panic!("{id}"))
    };
    let (receive, send) = (
        format!("node/{}/receive", node()),
        format!("node/{}/send", node()),
    );
    let one = link("party/sending/one", &receive);
    assert_eq!(
        (one.protocol.as_str(), one.volume, one.state),
        ("a", 3, Health::Fine)
    );
    let two = link("party/sending/two", &receive);
    assert_eq!(
        (two.protocol.as_str(), two.volume, two.state),
        ("b", 4, Health::Done)
    );
    assert_eq!(link(&send, "party/receiving/two").volume, 5);
}

#[test]
fn a_facing_party_faces_every_endpoint_and_the_whole_count() {
    let mut snapshot = Snapshot::new();
    health(&mut snapshot, "receive/a/json", Health::Fine);
    health(&mut snapshot, "receive/b/json", Health::Fine);
    count(&mut snapshot, "receive", Counted::Streams, 9);
    let topology = drawing(&snapshot, &[node().as_str()]);
    let transports = [
        ("a".to_string(), "file".to_string()),
        ("b".to_string(), "sftp".to_string()),
    ];
    let drawn = |transports| {
        let mut drawn = topology.clone();
        facing(
            &snapshot,
            &root(),
            &[node().as_str()],
            transports,
            &mut drawn,
        );
        drawn.links[0].clone()
    };
    // A running node's endpoints are its Locations: its link says the
    // transports their addresses chose and names the Locations only in
    // its evidence (ADR-0018, amendment 2026-11-11).
    let link = drawn(&transports);
    assert_eq!((link.protocol.as_str(), link.volume), ("2 transports", 9));
    assert!(link.evidence.contains("at a, b"), "{}", link.evidence);
    let one = [("a".to_string(), "file".to_string())];
    assert_eq!(
        drawn(&one).protocol,
        "file",
        "one Location facing a Party says its transport, never its name"
    );
}

#[test]
fn one_location_facing_a_party_gives_its_transport_never_its_name() {
    let mut snapshot = Snapshot::new();
    health(
        &mut snapshot,
        "receive/Sales orders from Party-A",
        Health::Fine,
    );
    let mut topology = drawing(&snapshot, &[node().as_str()]);
    let transports = [("Sales orders from Party-A".to_string(), "file".to_string())];
    facing(
        &snapshot,
        &root(),
        &[node().as_str()],
        &transports,
        &mut topology,
    );
    let link = &topology.links[0];
    assert_eq!(link.protocol, "file");
    assert!(
        link.evidence.contains("at Sales orders from Party-A"),
        "{}",
        link.evidence
    );
}

/// The owner, 2026-11-12: one Party per node facing outside, numbered per
/// side in the order the nodes stand, never by a name; and 2026-11-13, the
/// labels TX-Party and RX-Party. Three nodes, the second receiving only and the others
/// both: the TX-Parties sending into them are 1, 2 and 3, the RX-Parties
/// they deliver to 1 and 2, each id naming the node it faces, so a node's
/// own drawing and the cluster's meet.
#[test]
fn each_node_facing_outside_has_a_party_of_its_own_numbered_in_the_order_given() {
    let test = test_cluster();
    let [first, second, third] = [0, 1, 2].map(|at| test.node(at).name.clone());
    let mut snapshot = Snapshot::new();
    for (name, leaf) in [
        (&first, "receive/In"),
        (&first, "send/Out"),
        (&second, "receive/In"),
        (&third, "receive/In"),
        (&third, "send/Out"),
    ] {
        health_at(&mut snapshot, name, leaf, Health::Fine);
    }
    // The order given is the map's, here the reverse of the names'.
    let order = [third.as_str(), second.as_str(), first.as_str()];
    let mut topology = drawing(&snapshot, &order);
    facing(&snapshot, &root(), &order, &[], &mut topology);
    let party = |side: &str, name: &str| {
        let id = format!("party/{side}/node/{name}");
        topology
            .nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.label.clone())
    };
    assert_eq!(party("sending", &third), Some(format!("{TX_PARTY} 1")));
    assert_eq!(party("sending", &second), Some(format!("{TX_PARTY} 2")));
    assert_eq!(party("sending", &first), Some(format!("{TX_PARTY} 3")));
    assert_eq!(party("receiving", &third), Some(format!("{RX_PARTY} 1")));
    assert_eq!(party("receiving", &second), None, "it sends nothing out");
    assert_eq!(party("receiving", &first), Some(format!("{RX_PARTY} 2")));
}
