//! The mood of a scope, what it is called, what color it is painted, and
//! which of two records is the worse: observability-model.md section 6 and
//! ADR-0041, once.
//!
//! Every word, color name, rollup and order a surface shows for a mood is
//! decided here and nowhere else. The runtime's cdylib forwards each to a
//! surface over `xmip_operate.h` section 7 (`xmip_health_word_v1`,
//! `xmip_health_named_v1`, `xmip_health_color_v1`, `xmip_health_rolled_v1`,
//! `xmip_health_order_v1`), so `Xmip.Surface`, the PowerShell module and the
//! GUI call these rather than keep a table of their own (ADR-0052, amendment
//! 2026-09-24: one implementation, the surfaces call the runtime's exports).

use std::cmp::Ordering;

/// The mood of a scope — observability-model.md section 6. A mood, not a colour:
/// this names what a human gets out of a thread, process, node or cluster, and
/// a surface renders it however it likes (the GUI paints it). It is about the
/// resource under load, not the machine: it tells an operator whether results
/// are flowing and, when they are not, what to do — change the load, replace the
/// hardware, fix the one thing that is stuck (ADR-0041).
///
/// The **leaf** moods, in worsening order: `Fine` (results flowing), `Paused` (a
/// deliberate hold — an operator is working on it), `Working` (handling the
/// load), `Stressed` (strained — change the load), `Exhausted` (spent — replace
/// the hardware), `Done` (blocked or failed — the pain, a cert to renew, a
/// password, a missing folder).
///
/// `Holding` is the **rollup** mood, not a leaf's: in a perfect world everything
/// is `Fine`; the moment anything below is not, the parent is displeased and
/// reports `Holding` — drill in. So a parent is `Fine` or `Holding`, and a leaf
/// carries the real mood. `Fine` up the tree means every leaf beneath is `Fine`.
///
/// The declaration order is the ranking: a later mood is the worse one, and
/// [`Standing`] orders records by it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Health {
    Fine,
    /// A deliberate hold — an operator is working on it. Not a fault and not
    /// strain; it yields nothing because someone paused it on purpose.
    Paused,
    Working,
    Stressed,
    Exhausted,
    Done,
    /// Rollup only — a parent with something not-`Fine` beneath it.
    Holding,
}

impl Health {
    /// Every mood, best first: the leaf moods in worsening order, then the
    /// rollup.
    pub const ALL: [Health; 7] = [
        Health::Fine,
        Health::Paused,
        Health::Working,
        Health::Stressed,
        Health::Exhausted,
        Health::Done,
        Health::Holding,
    ];

    /// The mood as the word the estate uses, lower case: what a snapshot
    /// publishes, what a command prints and what a stylesheet class is.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Health::Fine => "fine",
            Health::Paused => "paused",
            Health::Working => "working",
            Health::Stressed => "stressed",
            Health::Exhausted => "exhausted",
            Health::Done => "done",
            Health::Holding => "holding",
        }
    }

    /// The mood a word names, exactly and in lower case, or `None` when no
    /// mood is called that.
    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|health| health.word() == word)
    }

    /// What a parent shows when this is the worst mood beneath it (ADR-0041):
    /// `Fine` when it is `Fine`, and `Holding` the moment it is anything
    /// else. A leaf's mood does not propagate; the leaf that owns the trouble
    /// keeps its own, and an operator drills down through the `Holding`
    /// scopes to it.
    #[must_use]
    pub const fn rolled(self) -> Self {
        match self {
            Health::Fine => Health::Fine,
            _ => Health::Holding,
        }
    }

    /// The name of the color a surface paints the mood in (ADR-0041,
    /// amendment 2026-09-14): the name, never the paint. A stylesheet renders
    /// each name to the estate's tokens and a console to its nearest color,
    /// so no two surfaces paint one mood two ways.
    #[must_use]
    pub const fn color(self) -> &'static str {
        match self {
            Health::Fine => "green",
            Health::Paused => "slate",
            Health::Working => "blue",
            Health::Stressed => "yellow",
            Health::Exhausted => "burnt",
            Health::Done => "red",
            Health::Holding => "orange",
        }
    }
}

/// Where one record stands in the worst-first order every reader returns
/// health in: the worse mood first, then the higher severity, then the scope
/// in byte order so that equals answer the same way every time.
///
/// The snapshot sorts by it, the operator boundary hands records out in it,
/// and a surface orders what it holds by asking `xmip_health_order_v1`,
/// which is [`Standing::worst_first`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Standing<'a> {
    pub health: Health,
    pub severity: u8,
    pub scope: &'a str,
}

impl Ord for Standing<'_> {
    /// `Less` is worse: a sort puts the worst record first.
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .health
            .cmp(&self.health)
            .then(other.severity.cmp(&self.severity))
            .then(self.scope.cmp(other.scope))
    }
}

impl PartialOrd for Standing<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Standing<'_> {
    /// Many records in the worst-first order, as the positions they hold in
    /// `standings`: the first index is the worst record's. Equals keep the
    /// order they came in. What a surface asks when it orders a whole
    /// publication at once (`xmip_health_order_v1`), rather than one pair at
    /// a time.
    #[must_use]
    pub fn worst_first(standings: &[Standing<'_>]) -> Vec<usize> {
        let mut order: Vec<usize> = (0..standings.len()).collect();
        order.sort_by(|&a, &b| standings[a].cmp(&standings[b]));
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standing(health: Health, severity: u8, scope: &str) -> Standing<'_> {
        Standing {
            health,
            severity,
            scope,
        }
    }

    #[test]
    fn every_mood_has_one_word_and_reads_back_from_it() {
        let words = Health::ALL.map(Health::word);

        assert_eq!(
            words,
            [
                "fine",
                "paused",
                "working",
                "stressed",
                "exhausted",
                "done",
                "holding"
            ]
        );
        for health in Health::ALL {
            assert_eq!(Health::named(health.word()), Some(health));
        }
    }

    #[test]
    fn a_word_is_exact_lower_case_and_anything_else_names_no_mood() {
        assert_eq!(Health::named("Fine"), None);
        assert_eq!(Health::named("DONE"), None);
        assert_eq!(Health::named(" fine"), None);
        assert_eq!(Health::named(""), None);
        assert_eq!(Health::named("unknown"), None);
    }

    #[test]
    fn every_mood_has_the_color_adr_0041_names() {
        assert_eq!(
            Health::ALL.map(Health::color),
            ["green", "slate", "blue", "yellow", "burnt", "red", "orange"]
        );
    }

    #[test]
    fn a_parent_is_fine_or_holding_and_nothing_else() {
        assert_eq!(
            Health::ALL.map(Health::rolled),
            [
                Health::Fine,
                Health::Holding,
                Health::Holding,
                Health::Holding,
                Health::Holding,
                Health::Holding,
                Health::Holding
            ]
        );
    }

    #[test]
    fn all_is_in_worsening_order_with_the_rollup_last() {
        assert!(Health::ALL.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(Health::ALL.last(), Some(&Health::Holding));
    }

    #[test]
    fn the_worse_mood_stands_first_whatever_the_severity() {
        let done = standing(Health::Done, 10, "xmip:///b");
        let stressed = standing(Health::Stressed, 99, "xmip:///a");

        assert_eq!(done.cmp(&stressed), Ordering::Less);
        assert_eq!(stressed.cmp(&done), Ordering::Greater);
    }

    #[test]
    fn within_a_mood_the_more_severe_then_the_first_scope_stands_first() {
        let severe = standing(Health::Done, 90, "xmip:///z");
        let mild = standing(Health::Done, 60, "xmip:///a");
        let first = standing(Health::Done, 60, "xmip:///a");
        let second = standing(Health::Done, 60, "xmip:///b");

        assert_eq!(severe.cmp(&mild), Ordering::Less);
        assert_eq!(first.cmp(&second), Ordering::Less);
        assert_eq!(first.cmp(&first), Ordering::Equal);
    }

    #[test]
    fn many_are_ordered_worst_first_by_position_and_equals_keep_theirs() {
        let records = [
            standing(Health::Fine, 0, "xmip:///a"),
            standing(Health::Done, 60, "xmip:///d"),
            standing(Health::Holding, 0, "xmip:///h"),
            standing(Health::Fine, 0, "xmip:///a"),
            standing(Health::Done, 90, "xmip:///e"),
        ];

        assert_eq!(Standing::worst_first(&records), [2, 4, 1, 0, 3]);
        assert!(Standing::worst_first(&[]).is_empty());
    }

    #[test]
    fn a_sort_puts_the_worst_first() {
        let mut records = [
            standing(Health::Fine, 0, "xmip:///a"),
            standing(Health::Holding, 0, "xmip:///h"),
            standing(Health::Done, 60, "xmip:///d"),
            standing(Health::Done, 90, "xmip:///e"),
            standing(Health::Paused, 30, "xmip:///p"),
        ];
        records.sort();

        assert_eq!(
            records.map(|record| record.scope),
            [
                "xmip:///h",
                "xmip:///e",
                "xmip:///d",
                "xmip:///p",
                "xmip:///a"
            ]
        );
    }
}
