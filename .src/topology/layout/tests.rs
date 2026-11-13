use std::collections::BTreeMap;

use configure::fixture::test_cluster;
use node::{Capability, NodeRole};

use super::{Band, CLEARANCE, Column, Place, clearance, layout, lines, place_order};
use crate::health::Health;
use crate::snapshot::{HealthRecord, Snapshot};
use crate::topology::draw::{CLUSTER, members};
use crate::topology::party::{self, Party};
use crate::topology::relate::{Member, relations};
use crate::topology::{Origin, Pattern, Relation, Topology, TopologyLink, link_id};

fn root() -> String {
    test_cluster().scope()
}

/// The test cluster's members as its file declares them, `renamed`, the
/// Storage nodes placed in `order` (their places in the file's order, or
/// reversed).
fn cluster(renamed: &dyn Fn(&str) -> String, reversed: bool) -> Vec<Member> {
    let nodes = test_cluster().nodes;
    let storing = nodes
        .iter()
        .filter(|node| node.roles.iter().any(|role| role == "storage"))
        .count();
    let mut place = 0;
    nodes
        .iter()
        .map(|node| {
            let roles: Vec<NodeRole> = node
                .roles
                .iter()
                .filter_map(|word| NodeRole::named(word))
                .collect();
            let listed = roles.contains(&NodeRole::Storage).then(|| {
                place += 1;
                let at = place - 1;
                if reversed {
                    u32::try_from(storing).unwrap_or(0) - 1 - at
                } else {
                    at
                }
            });
            Member::new(&renamed(&node.name), &roles, listed)
        })
        .collect()
}

/// The cluster of `of` drawn as a roll draws it: every member saying what
/// it declares, related by role, and its Parties — one facing each node, or
/// `named` Parties each on every side.
fn drawn(of: &[Member], named: usize) -> Topology {
    let root = root();
    let mut snapshot = Snapshot::new();
    for member in of {
        snapshot.record_health(HealthRecord {
            scope: format!("{root}/node/{}/capability", member.node),
            health: Health::Fine,
            severity: 0,
            evidence: Capability::of(&member.roles).evidence(),
            observed_unix_nanos: 7,
        });
    }
    let mut topology = Topology {
        nodes: members(&snapshot, &root, of),
        links: relations(&root, of, &snapshot),
        ..Topology::default()
    };
    let ordered: Vec<&str> = place_order(of)
        .into_iter()
        .map(|member| member.node.as_str())
        .collect();
    if named == 0 {
        party::facing(&snapshot, &root, &ordered, &[], &mut topology);
    } else {
        let names: Vec<String> = (1..=named).map(|at| format!("party-{at}")).collect();
        let parties: Vec<Party<'_>> = names
            .iter()
            .map(|name| Party {
                name,
                sends: None,
                receives: None,
            })
            .collect();
        party::draw(&snapshot, &root, &ordered, &parties, &mut topology);
    }
    topology
}

/// Where each thing stands, by its id.
fn stands(topology: &Topology) -> BTreeMap<String, Place> {
    layout(topology, CLUSTER, &[])
        .into_iter()
        .map(|place| (place.id.clone(), place))
        .collect()
}

fn same() -> impl Fn(&str) -> String {
    |name: &str| name.to_string()
}

#[test]
fn each_node_stands_in_the_band_and_column_its_roles_give_it() {
    let of = cluster(&same(), false);
    let places = stands(&drawn(&of, 0));
    for member in &of {
        let at = &places[&format!("node/{}", member.node)];
        let expected = if member.declares(NodeRole::Storage) {
            if member.storage_place == Some(0) {
                (Band::Path, Column::Storage)
            } else {
                (Band::Standby, Column::Storage)
            }
        } else if member.declares(NodeRole::Receiving) {
            (Band::Path, Column::Receiving)
        } else if member.declares(NodeRole::Sending) {
            (Band::Path, Column::Sending)
        } else if member.declares(NodeRole::Processing) {
            (Band::Processing, Column::Storage)
        } else if member.declares(NodeRole::Executing) {
            (Band::Executing, Column::Spanning)
        } else {
            (Band::Operation, Column::Spanning)
        };
        assert_eq!((at.band, at.column), expected, "{}", member.node);
        assert!((0.0..=1.0).contains(&at.x) && (0.0..=1.0).contains(&at.y));
    }
    let party = places
        .values()
        .filter(|place| place.column == Column::TxParty)
        .count();
    assert!(party > 0, "{places:?}");
}

/// The renamed cluster — receiving nodes given the Storage nodes' names,
/// the rest strings with no letters to read — stands every node in the same
/// band and column.
#[test]
fn a_renamed_cluster_stands_the_same() {
    let of = cluster(&same(), false);
    let receiving: Vec<String> = of
        .iter()
        .filter(|member| member.declares(NodeRole::Receiving))
        .map(|member| member.node.clone())
        .collect();
    let storing: Vec<String> = of
        .iter()
        .filter(|member| member.declares(NodeRole::Storage))
        .map(|member| member.node.clone())
        .collect();
    let mut renaming: BTreeMap<String, String> = BTreeMap::new();
    for (one, other) in receiving.iter().zip(&storing) {
        renaming.insert(one.clone(), other.clone());
        renaming.insert(other.clone(), one.clone());
    }
    for (at, member) in of.iter().enumerate() {
        renaming
            .entry(member.node.clone())
            .or_insert_with(|| format!("{at}-x{}", 31 * at));
    }
    let first = stands(&drawn(&of, 0));
    let again = stands(&drawn(&cluster(&|name| renaming[name].clone(), false), 0));
    for (old, new) in &renaming {
        let (one, other) = (
            &first[&format!("node/{old}")],
            &again[&format!("node/{new}")],
        );
        assert_eq!(
            (one.band, one.column),
            (other.band, other.column),
            "{old} as {new}"
        );
    }
}

#[test]
fn swapped_roles_swap_places_and_the_listed_order_stacks_storage() {
    let of = cluster(&same(), false);
    let pick = |role: NodeRole| {
        of.iter()
            .find(|member| member.declares(role))
            .map(|member| member.node.clone())
            .unwrap_or_default()
    };
    let (receiving, sending) = (pick(NodeRole::Receiving), pick(NodeRole::Sending));
    let swapped: Vec<Member> = of
        .iter()
        .map(|member| {
            let roles = if member.node == receiving {
                vec![NodeRole::Sending]
            } else if member.node == sending {
                vec![NodeRole::Receiving]
            } else {
                member.roles.clone()
            };
            Member::new(&member.node, &roles, member.storage_place)
        })
        .collect();
    let (first, again) = (stands(&drawn(&of, 0)), stands(&drawn(&swapped, 0)));
    let column =
        |places: &BTreeMap<String, Place>, name: &str| places[&format!("node/{name}")].column;
    assert_eq!(column(&again, &receiving), column(&first, &sending));
    assert_eq!(column(&again, &sending), column(&first, &receiving));

    let reversed = stands(&drawn(&cluster(&same(), true), 0));
    for member in of
        .iter()
        .filter(|member| member.declares(NodeRole::Storage))
    {
        let id = format!("node/{}", member.node);
        let (band, now) = (first[&id].band, reversed[&id].band);
        let expected = if band == Band::Path {
            Band::Standby
        } else {
            Band::Path
        };
        assert_eq!(now, expected, "{}", member.node);
    }
}

/// The map stands still while the pulses move: the same drawing with
/// volume, rates and traffic no configuration declares lays out the same.
#[test]
fn traffic_never_moves_a_node() {
    let of = cluster(&same(), false);
    let quiet = drawn(&of, 0);
    let mut busy = quiet.clone();
    for (at, link) in busy.links.iter_mut().enumerate() {
        link.volume = 1000 + u64::try_from(at).unwrap_or(0);
        link.rate = 12.5;
        if link.origin == Origin::Configured {
            link.origin = Origin::Both;
        }
    }
    let (one, other) = (&of[0].node, &of[of.len() - 1].node);
    let (from, to) = (format!("node/{one}"), format!("node/{other}"));
    busy.links.push(TopologyLink {
        id: link_id(&from, &to, Relation::Traffic),
        from,
        to,
        pattern: Pattern::Streaming,
        origin: Origin::Observed,
        relation: Relation::Traffic,
        protocol: String::new(),
        state: Health::Fine,
        volume: 77,
        rate: 3.0,
        latency_ms: 0.0,
        progress: 0.0,
        attempts: 0,
        evidence: "not configured".to_string(),
    });
    assert_eq!(layout(&quiet, CLUSTER, &[]), layout(&busy, CLUSTER, &[]));
}

/// No line runs through a thing it does not join: the test cluster with a
/// Party facing each node, and with one to three Parties each facing every
/// node on each side.
#[test]
fn no_line_runs_through_a_node_it_does_not_join() {
    let of = cluster(&same(), false);
    for named in 0..=3 {
        let topology = drawn(&of, named);
        let places = layout(&topology, CLUSTER, &[]);
        let shown: Vec<&crate::topology::TopologyNode> = topology
            .nodes
            .iter()
            .filter(|node| node.parent == CLUSTER)
            .collect();
        let drawn = lines(&topology, &shown);
        let near: Vec<(&str, &str, f64)> = drawn
            .iter()
            .map(|line| {
                (
                    line.0,
                    line.1,
                    clearance(&places, std::slice::from_ref(line)),
                )
            })
            .filter(|(.., clear)| *clear < CLEARANCE)
            .collect();
        assert!(
            near.is_empty(),
            "{named} Parties a side: {near:?} {places:?}"
        );
    }
}

#[test]
fn a_drill_lays_out_receive_work_process_and_send_in_path_order() {
    let of = cluster(&same(), false);
    let executing = of
        .iter()
        .find(|member| member.declares(NodeRole::Executing))
        .map(|member| format!("node/{}", member.node))
        .unwrap_or_default();
    let inside = layout(&drawn(&of, 0), &executing, &[]);
    let order: Vec<(&str, Column)> = {
        let mut sorted: Vec<&Place> = inside.iter().collect();
        sorted.sort_by(|one, other| one.x.total_cmp(&other.x));
        sorted
            .into_iter()
            .map(|place| {
                (
                    place.id.rsplit('/').next().unwrap_or_default(),
                    place.column,
                )
            })
            .collect()
    };
    assert_eq!(
        order,
        [
            ("receive", Column::Receiving),
            ("process", Column::Storage),
            ("send", Column::Sending)
        ]
    );
    let only = layout(&drawn(&of, 0), &executing, &[&format!("{executing}/send")]);
    assert_eq!(only.len(), 1, "only what is shown");
}
