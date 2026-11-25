//! A Work Process's Instances, as the node that runs it publishes them
//! (`runtime-model.md` section 22; ADR-0066, amendment 2026-11-25; open
//! problem 192; `xmip_operate.h` section 29).
//!
//! The owner, 2026-11-25: *A Work Process is like an Orchestration in
//! BizTalk. So it is aware via subscriptions, even when running or not.*
//! So every surface shows which Instances each Work Process has, as
//! BizTalk's console shows running, dehydrated and suspended ones: each
//! running, waiting — for a Message of its key in each Correlation it
//! holds, until its earliest deadline — failed, with its code and why and
//! the Journey whose turn failed it, or completed; its own values; and its
//! history of Messages. Beside them, the Messages that failed in its queue,
//! each by its Journey: what an operator's Retry and Dismiss name (noun
//! `message`, as at a Send Port).
//!
//! **A window, newest first.** A node publishes the newest Instances it
//! knows of each Work Process it runs — read from the Ledger as it starts,
//! then as its own turns write them — and the rest are read from the Ledger
//! (`xmip_instances_v1`).

use serde::{Deserialize, Serialize};

use crate::failed_message::FailedMessage;

/// One Work Process's Instances on one node, as published.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkProcessInstances {
    /// The node that runs it: `xmip:///<cluster>/node/<name>`.
    #[serde(default)]
    pub node: String,
    /// The Work Process, by its configured name.
    #[serde(default)]
    pub work_process: String,
    /// Its newest Instances, newest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instances: Vec<WorkProcessInstance>,
    /// How many Messages that failed wait in its queue now.
    #[serde(default)]
    pub failed_count: u64,
    /// The oldest of them, oldest first, each by its Journey.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<FailedMessage>,
}

/// One Instance of a Work Process.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkProcessInstance {
    /// Its identifier: the Journey of the Message that started it.
    #[serde(default)]
    pub instance: String,
    /// `running`, `waiting`, `failed`, `completed`, or `dismissed` where an
    /// operator dismissed the Message that failed it.
    #[serde(default)]
    pub state: String,
    /// What it waits for: `message`, `time`, both as `message or time`, or
    /// nothing.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub waits_for: String,
    /// Its earliest deadline, where a timer waits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until_unix_nanos: Option<i64>,
    /// Each Correlation it holds a key in, and the key: what a Message must
    /// carry to be handed to it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub correlations: Vec<[String; 2]>,
    /// Where it failed: the code no scope caught, why, and the Journey whose
    /// turn failed it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub code: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub failed_by: String,
    /// Its own values, by name, as it keeps them across Messages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<[String; 2]>,
    /// The Messages it handled, oldest first, the newest it keeps.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<InstanceHandled>,
    /// How many entries of its history came before those kept.
    #[serde(default)]
    pub earlier: u64,
    /// How many turns it has taken.
    #[serde(default)]
    pub turns: u64,
    /// When Xmip Storage last wrote it.
    #[serde(default)]
    pub updated_unix_nanos: i64,
}

/// One entry of an Instance's history.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceHandled {
    #[serde(default)]
    pub at_unix_nanos: i64,
    /// `started`, `arrived`, `due`, `made`, `published`, `failed`,
    /// `retried` or `dismissed`.
    #[serde(default)]
    pub what: String,
    /// The Message, by its identifier.
    #[serde(default)]
    pub message: String,
    /// The Journey of the turn, by its identifier.
    #[serde(default)]
    pub journey: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use configure::fixture::test_cluster;

    fn instance(id: &str, turns: u64, updated: i64) -> WorkProcessInstance {
        WorkProcessInstance {
            instance: id.to_string(),
            state: "waiting".to_string(),
            waits_for: "message or time".to_string(),
            until_unix_nanos: Some(9),
            correlations: vec![["Order".to_string(), "A-1001".to_string()]],
            values: vec![["Delivered".to_string(), "2".to_string()]],
            history: vec![InstanceHandled {
                at_unix_nanos: 1,
                what: "started".to_string(),
                message: "m".to_string(),
                journey: "j".to_string(),
            }],
            turns,
            updated_unix_nanos: updated,
            ..WorkProcessInstance::default()
        }
    }

    #[test]
    fn a_work_processs_instances_read_back_as_written() {
        let written = WorkProcessInstances {
            node: test_cluster().node_scope(0),
            work_process: "Order in parts".to_string(),
            instances: vec![instance("i-1", 3, 5)],
            failed_count: 1,
            failed: vec![FailedMessage {
                journey: "j-2".to_string(),
                sequence: 4,
                reason: "Order in parts threw offline: the unit".to_string(),
            }],
        };
        let text = toml::to_string(&written).expect("written");
        let read: WorkProcessInstances = toml::from_str(&text).expect("read");
        assert_eq!(read, written);
    }
}
