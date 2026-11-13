use super::*;
use configure::fixture::test_cluster;

fn drawn(id: &str, label: &str) -> TopologyNode {
    TopologyNode {
        id: id.to_string(),
        parent: String::new(),
        label: label.to_string(),
        kind: NodeKind::Node,
        scope: String::new(),
        state: Health::Fine,
        origin: Origin::Both,
        load: 0.0,
        activity: 0.0,
        evidence: String::new(),
        roles: String::new(),
        place: None,
    }
}

fn linked(from: &str, to: &str, volume: u64) -> TopologyLink {
    TopologyLink {
        id: link_id(from, to, Relation::Traffic),
        from: from.to_string(),
        to: to.to_string(),
        pattern: Pattern::PublishConsume,
        origin: Origin::Observed,
        relation: Relation::Traffic,
        protocol: XMIP_STORAGE.to_string(),
        state: Health::Fine,
        volume,
        rate: 0.0,
        latency_ms: 0.0,
        progress: 0.0,
        attempts: 0,
        evidence: String::new(),
    }
}

#[test]
fn a_nodes_own_drawing_wins_for_what_it_owns_but_a_worse_mood_and_its_links_join() {
    let test = test_cluster();
    let (receiver, storer) = (&test.node(0).name, &test.node(1).name);
    let (receiving, storing) = (format!("node/{receiver}"), format!("node/{storer}"));
    let stage = format!("{receiving}/receive");
    let location = format!("{stage}/Sales orders from Party-A");
    let party = format!("party/sending/{receiving}");
    let struck = TopologyNode {
        state: Health::Holding,
        evidence: "a document no Subscription takes".to_string(),
        ..drawn(&location, "the roll's")
    };
    let mut cluster = Topology {
        nodes: vec![
            drawn("cluster", "the roll's"),
            TopologyNode {
                roles: "receiving".to_string(),
                ..drawn(&receiving, "the roll's")
            },
            TopologyNode {
                place: Some(1),
                ..drawn(&storing, "the roll's")
            },
            struck,
        ],
        links: vec![linked(&party, &stage, 3)],
        ..Topology::default()
    };
    let storage = Topology {
        nodes: vec![
            drawn("cluster", "the node's"),
            drawn(&storing, "its own"),
            drawn(&receiving, "drawn by another"),
        ],
        links: vec![linked(&receiving, &storing, 7)],
        ..Topology::default()
    };
    let own_drawing = Topology {
        nodes: vec![
            drawn(&receiving, "its own"),
            drawn(&stage, "Receive"),
            drawn(&location, "its own"),
        ],
        links: vec![linked(&party, &stage, 9)],
        ..Topology::default()
    };

    cluster.merge(storer, &storage);
    cluster.merge(receiver, &own_drawing);

    let found = |id: &str| {
        cluster
            .nodes
            .iter()
            .find(|one| one.id == id)
            .unwrap_or_else(|| panic!("{id}"))
    };
    assert_eq!(found("cluster").label, "the roll's", "no node owns it");
    assert_eq!(found(&storing).label, "its own");
    assert_eq!(
        found(&storing).place,
        Some(1),
        "a place the node did not say stays the configured one"
    );
    assert_eq!(
        found(&receiving).label,
        "its own",
        "only the node itself, never a node another drew"
    );
    assert_eq!(found(&receiving).roles, "receiving");
    assert_eq!(found(&stage).label, "Receive", "beneath it, added");
    let location = found(&location);
    assert_eq!(location.label, "its own");
    assert_eq!(
        (location.state, location.evidence.as_str()),
        (Health::Holding, "a document no Subscription takes"),
        "the reader's worse mood stays, with what it said"
    );
    let volumes: Vec<u64> = cluster.links.iter().map(|link| link.volume).collect();
    assert_eq!(
        volumes,
        [9, 7],
        "the node's own Party link wins; the Storage node's traffic joins"
    );
}

/// ADR-0052, amendment 2026-11-13: a relation configured and the traffic
/// observed over it share one id and merge to configured and observed, the
/// observed figures kept, whichever drawing is merged into the other.
#[test]
fn a_configured_relation_and_the_traffic_over_it_join_to_both_in_either_order() {
    let test = test_cluster();
    let (writer, storer) = (&test.node(2).name, &test.node(0).name);
    let (from, to) = (draw::node_id(writer), draw::node_id(storer));
    let relation = |origin, volume| TopologyLink {
        origin,
        relation: Relation::WritesInto,
        id: link_id(&from, &to, Relation::WritesInto),
        ..linked(&from, &to, volume)
    };
    let drawing = |link: TopologyLink| Topology {
        links: vec![link],
        ..Topology::default()
    };
    let configured = drawing(relation(Origin::Configured, 0));
    let observed = drawing(relation(Origin::Observed, 12));

    let mut roll = configured.clone();
    roll.merge(storer, &observed);
    let mut other_way = observed;
    other_way.merge(writer, &configured);
    for merged in [roll, other_way] {
        assert_eq!(merged.links.len(), 1, "{merged:?}");
        assert_eq!(
            (merged.links[0].origin, merged.links[0].volume),
            (Origin::Both, 12)
        );
    }
    assert_ne!(
        link_id(&from, &to, Relation::WritesInto),
        link_id(&from, &to, Relation::OperatesAndMonitors),
        "two relations over one pair in one direction are two links"
    );
}

#[test]
fn a_node_crosses_toml_by_its_words() {
    let cluster = test_cluster();
    let name = &cluster.node(0).name;
    let node = TopologyNode {
        id: format!("node/{name}"),
        parent: "cluster".to_string(),
        label: name.clone(),
        kind: NodeKind::Node,
        scope: cluster.node_scope(0),
        state: Health::Stressed,
        origin: Origin::Configured,
        load: 0.0,
        activity: 2.5,
        evidence: String::new(),
        roles: "operational,monitoring".to_string(),
        place: Some(0),
    };
    let text = toml::to_string(&node).expect("writes");
    assert!(text.contains("kind = \"node\""), "{text}");
    assert!(text.contains("state = \"stressed\""), "{text}");
    assert!(text.contains("origin = \"configured\""), "{text}");
    assert!(
        text.contains("roles = \"operational,monitoring\""),
        "{text}"
    );
    assert!(text.contains("place = 0"), "{text}");
    assert_eq!(toml::from_str::<TopologyNode>(&text), Ok(node));

    let strange: TopologyNode =
        toml::from_str("kind = \"mainframe\"\nstate = \"sulking\"").expect("reads");
    assert_eq!(strange.kind, NodeKind::Computer);
    assert_eq!(strange.state, Health::Stressed);
    assert_eq!(strange.origin, Origin::Both);
    assert_eq!((strange.roles.as_str(), strange.place), ("", None));
    let link: TopologyLink = toml::from_str("relation = \"befriends\"").expect("reads");
    assert_eq!(
        link.relation,
        Relation::Traffic,
        "an unknown relation is traffic"
    );
}
