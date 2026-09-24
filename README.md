# xmip-core-observe

Observation: what is happening now, and what is unhealthy. What the message
path writes lands in a `Snapshot` over the scope tree, with its activity and
history, and the operator boundary in `xmip_operate.h` reads that snapshot
and nothing else (ADR-0027 clause 6).

Observation is near-real-time and never synchronous: Receive, Process and
Send never wait for it. It is not the durable record — audit is — and it does
not paint: health is a mood (Fine, Paused, Working, Stressed, Exhausted,
Done, with Holding the rollup a parent shows), and a surface gives it a color
(ADR-0041).

A scope is an Xmip URI, and the tree is its path (ADR-0027 clauses 3 and 4).
`Scope` reads one — the scheme and the authority go, a slash at either end
is ignored, empty text is the root — and `Scope::contains` is the one
containment rule the snapshot and the activity log answer by: at and beneath,
by segment, never by character. `ScopeTree` in `Xmip.Surface` writes the same
rule again, on purpose, because a surface loads no Rust library; both are
held to `src/scope-vector.toml`, which a test on each side reads (ADR-0052,
amendment 2026-09-24).

`doc/architecture/observability-model.md` sections 6 and 7 govern it; each
exporter is a technology under this repository, and `architecture.toml` names
them.
