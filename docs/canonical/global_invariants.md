## Global invariants (apply to every rig)

These are not “features.” They are certifiability gates that every rig must satisfy.
1.	Deterministic replay
	Same request payload + same Sterling version + same config ⇒ identical trace bundle hash. No hidden randomness unless explicitly modeled as stochastic outcomes.
2.	Typed operators, fail-closed legality
	Every operator has a type, preconditions, and effects. If legality cannot be proven from state + operator, it is treated as illegal (fail-closed).
3.	Canonical state hashing + equivalence reduction
	State hashing must be stable under irrelevant variation. Count capping and symmetry/equivalence classes are required to prevent memory blow-up and preserve transfer.
4.	Learning never changes semantics
	Learning may change ordering/priors and cost estimates if you explicitly model them, but must not invent transitions or silently alter preconditions/effects.
5.	Credit assignment is execution-grounded
	Planner “success” is not reinforcement. Only executed outcomes (step-level success/failure reports) update priors. Partial credit updates only the responsible segment.
6.	Audit-grade explanations
	Every solve emits: constraints that bound the solution, legality gates considered, top competing alternatives and why rejected, and the evidence/experience that shaped priors.
7.	Rule injection hardening
	Client-defined rules/operators are untrusted input. Strict validation, boundedness limits, schema/version gating, and semantic guards are mandatory.
8.	Multi-objective handling is explicit
	If time vs risk vs resource burn tradeoffs exist, represent them explicitly (weighted scalar with declared weights, or Pareto set). Never smuggle objectives into ad hoc heuristics.