//! Advice: what the operations surfaces suggest, the one shape every
//! subject's advice takes and every surface shows (the owner, 2026-10-22:
//! *We have to be lenient to people that does not know all languages, the
//! operations views just have to report it, suggest improvements*).
//! Nothing advised is refused: advice says what is costly and what would do
//! the same work for less, in words, beside the figures it was read from.
//!
//! One function per subject builds it. A step's cost per Message is this
//! file's ([`step_cost`]); others place theirs beside it.

use std::time::Duration;

/// One piece of advice: where it concerns, what it says, and the figures
/// it was read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Advice {
    /// The Location, Port or step it concerns, as an Xmip scope's text.
    pub scope: String,
    /// What it says, in a sentence a person reads.
    pub said: String,
    /// Each figure it was read from, by name, with its value as written.
    pub evidence: Vec<(String, String)>,
}

/// How a step's unit does its work, as far as advice needs to know.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Working {
    /// Compiled code: a standard unit, a designer's map, a user's own.
    Compiled,
    /// A script, run by a runtime the configuration invites.
    Script,
    /// A stylesheet, applied by an XSLT engine.
    Stylesheet,
}

/// What a step costs per Message, beyond which it is advised on: the
/// millisecond rule's (CONTRIBUTING.md).
pub const COSTLY: Duration = Duration::from_millis(1);

/// What a compiled unit is said to do the same work in.
const COMPILED: &str = "about 1 ms";

/// The advice on the step at `scope`, of `capability` (`transformation`,
/// `preparation`), working as `working` and costing `per_message`, where
/// it is costly and something faster does the same work.
#[must_use]
pub fn step_cost(
    scope: &str,
    capability: &str,
    working: Working,
    per_message: Duration,
) -> Option<Advice> {
    if per_message <= COSTLY {
        return None;
    }
    let taken = written(per_message);
    let said = match working {
        Working::Compiled => return None,
        Working::Script => format!(
            "this script {capability} takes {taken} per Message; a compiled unit would do it \
             in {COMPILED}"
        ),
        Working::Stylesheet => format!(
            "this stylesheet {capability} takes {taken} per Message; an engine that compiles \
             ahead of time, or a compiled map, would do it in {COMPILED}"
        ),
    };
    Some(Advice {
        scope: scope.to_string(),
        said,
        evidence: vec![("per_message".to_string(), taken)],
    })
}

/// A cost as a person reads it: whole milliseconds from ten up, tenths
/// below.
fn written(cost: Duration) -> String {
    let millis = cost.as_secs_f64() * 1000.0;
    if millis >= 10.0 {
        format!("{millis:.0} ms")
    } else {
        format!("{millis:.1} ms")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use configure::fixture::test_cluster;

    /// A step's scope in the test cluster.
    fn step() -> String {
        format!(
            "{}/ReceivePort/Orders/ReceiveLocation/OrdersIn",
            test_cluster().scope()
        )
    }

    #[test]
    fn a_costly_script_is_advised_to_be_compiled_and_nothing_is_refused() {
        let step = step();
        let advice = step_cost(
            &step,
            "transformation",
            Working::Script,
            Duration::from_millis(280),
        )
        .expect("advised");
        assert_eq!(
            advice.said,
            "this script transformation takes 280 ms per Message; a compiled unit would do it \
             in about 1 ms"
        );
        assert_eq!(advice.scope, step);
        assert_eq!(
            advice.evidence,
            [("per_message".to_string(), "280 ms".to_string())]
        );
        let stylesheet = step_cost(
            &step,
            "transformation",
            Working::Stylesheet,
            Duration::from_micros(4200),
        );
        assert!(stylesheet.is_some_and(|advice| advice.said.contains("4.2 ms per Message")));
        assert_eq!(
            step_cost(
                &step,
                "transformation",
                Working::Script,
                Duration::from_micros(900)
            ),
            None
        );
        assert_eq!(
            step_cost(
                &step,
                "preparation",
                Working::Compiled,
                Duration::from_secs(2)
            ),
            None
        );
    }
}
