use configure::fixture::test_cluster;

use super::*;
use crate::health::Health;
use crate::snapshot::HealthRecord;
use crate::subscription::Subscription;
use crate::topology::{Origin, Pattern, TopologyNode};

const LOCATION: &str = "Sales orders from Party-A";
const SUBSCRIPTION: &str = "Sales orders";
const PORT: &str = "Sales orders to ERP";

fn linked(from: &str, to: &str, protocol: &str) -> TopologyLink {
    TopologyLink {
        id: super::super::link_id(from, to, crate::topology::Relation::Traffic),
        from: from.to_string(),
        to: to.to_string(),
        pattern: Pattern::PublishConsume,
        origin: Origin::Observed,
        relation: crate::topology::Relation::Traffic,
        protocol: protocol.to_string(),
        state: Health::Fine,
        volume: 1,
        rate: 0.0,
        latency_ms: 0.0,
        progress: 0.0,
        attempts: 0,
        evidence: String::new(),
    }
}

/// Three nodes: one receiving, routing and writing into the Storage node,
/// one Storage node, and one sending the Send Port the Subscription reaches.
fn path() -> (Snapshot, Topology, [String; 3]) {
    let cluster = test_cluster();
    let [receiving, storing, sending] = [0, 1, 2].map(|at| cluster.node(at).name.clone());
    let mut snapshot = Snapshot::new();
    for (on, leaf) in [
        (0, format!("receive/{LOCATION}")),
        (2, format!("send/{PORT}")),
    ] {
        snapshot.record_health(HealthRecord {
            scope: format!("{}/{leaf}", cluster.node_scope(on)),
            health: Health::Fine,
            severity: 0,
            evidence: String::new(),
            observed_unix_nanos: 1,
        });
    }
    snapshot.record_subscription(Subscription {
        node: cluster.node_scope(0),
        name: SUBSCRIPTION.to_string(),
        send_ports: vec![PORT.to_string()],
        ..Subscription::default()
    });
    let at = |name: &str, beneath: &str| format!("node/{name}{beneath}");
    // Each Party facing one node, as a running node draws it, said by what
    // it is drawn as.
    let (into, out) = (
        format!("party/sending/{}", at(&receiving, "")),
        format!("party/receiving/{}", at(&sending, "")),
    );
    let party = |id: &str, label: &str| TopologyNode {
        id: id.to_string(),
        parent: "cluster".to_string(),
        label: label.to_string(),
        kind: crate::topology::NodeKind::Party,
        scope: String::new(),
        state: Health::Fine,
        origin: Origin::Both,
        load: 0.0,
        activity: 0.0,
        evidence: String::new(),
        roles: String::new(),
        place: None,
    };
    let topology = Topology {
        nodes: vec![party(&into, "TX-Party 1"), party(&out, "RX-Party 1")],
        links: vec![
            linked(&into, &at(&receiving, "/receive"), "file"),
            linked(
                &at(&receiving, &format!("/receive/{LOCATION}")),
                &at(&receiving, &format!("/process/{SUBSCRIPTION}")),
                "",
            ),
            linked(&at(&receiving, ""), &at(&storing, ""), XMIP_STORAGE),
            linked(&at(&storing, ""), &at(&sending, ""), XMIP_STORAGE),
            linked(&at(&sending, "/send"), &out, "file"),
        ],
        ..Topology::default()
    };
    (snapshot, topology, [receiving, storing, sending])
}

fn kinds(found: &FromTo) -> Vec<(&str, &str, &str)> {
    found
        .hops
        .iter()
        .map(|hop| (hop.kind, hop.name.as_str(), hop.node.as_str()))
        .collect()
}

#[test]
fn a_receive_location_goes_from_its_party_through_storage_to_the_send_port_and_its_party() {
    let (snapshot, topology, [receiving, storing, sending]) = path();
    let scope = format!(
        "{}/receive/{LOCATION}/delivery",
        test_cluster().node_scope(0)
    );

    let found = from_to(&snapshot, Some(&topology), &scope);

    assert_eq!(
        kinds(&found),
        [
            ("Sending Party", "TX-Party 1", ""),
            ("Receive Location", LOCATION, receiving.as_str()),
            ("Subscription", SUBSCRIPTION, receiving.as_str()),
            ("Storage node", storing.as_str(), storing.as_str()),
            ("Send Port", PORT, sending.as_str()),
            ("Receiving Party", "RX-Party 1", ""),
        ]
    );
    assert_eq!(found.from, "TX-Party 1 over file");
    assert_eq!(found.to, "RX-Party 1 over file");
    assert_eq!(found.hops[4].words(), format!("{PORT} [{sending}]"));
    assert_eq!(
        found.line,
        "from TX-Party 1 over file → to RX-Party 1 over file"
    );
    assert!(
        found.path.starts_with(&format!(
            "TX-Party 1 over file → {LOCATION} [{receiving}] → "
        )),
        "{}",
        found.path
    );
}

#[test]
fn a_send_port_comes_from_the_subscription_that_reaches_it_on_another_node() {
    let (snapshot, topology, [receiving, storing, sending]) = path();
    let scope = format!("{}/send/{PORT}", test_cluster().node_scope(2));

    let found = from_to(&snapshot, Some(&topology), &scope);

    assert_eq!(
        kinds(&found),
        [
            ("Sending Party", "TX-Party 1", ""),
            ("Receive Location", LOCATION, receiving.as_str()),
            ("Subscription", SUBSCRIPTION, receiving.as_str()),
            ("Storage node", storing.as_str(), storing.as_str()),
            ("Send Port", PORT, sending.as_str()),
            ("Receiving Party", "RX-Party 1", ""),
        ]
    );
}

#[test]
fn a_node_goes_from_what_hands_it_work_to_what_it_hands_work_to() {
    let (snapshot, topology, [receiving, storing, _]) = path();
    let scope = test_cluster().node_scope(0);

    let found = from_to(&snapshot, Some(&topology), &scope);

    assert_eq!(
        kinds(&found),
        [
            ("Sending Party", "TX-Party 1", ""),
            ("Node", receiving.as_str(), receiving.as_str()),
            ("Node", storing.as_str(), storing.as_str()),
        ]
    );
    assert_eq!(found.from, "TX-Party 1 over file");
    assert_eq!(found.to, format!("{storing} [{storing}]"));
}

#[test]
fn a_scope_on_no_node_has_no_from_or_to_and_an_unrouted_entry_goes_to_no_subscription() {
    let (snapshot, topology, _) = path();
    let found = from_to(&snapshot, Some(&topology), &test_cluster().scope());
    assert_eq!((found.from.as_str(), found.to.as_str()), ("", ""));
    assert!(found.hops.is_empty() && found.line.is_empty() && found.path.is_empty());
    assert_eq!(found.unrouted, "no Subscription");
    let undrawn = from_to(
        &snapshot,
        None,
        &format!("{}/receive/{LOCATION}", test_cluster().node_scope(0)),
    );
    assert_eq!(
        kinds(&undrawn).first().map(|hop| hop.0),
        Some("Receive Location"),
        "nothing guessed where nothing was drawn"
    );
}
