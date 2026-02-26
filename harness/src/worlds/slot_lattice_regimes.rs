//! Canonical regime constructors for `SlotLatticeSearch`.
//!
//! Each regime returns `(world, policy, expectations)` as a matched triple.
//! Tests should ONLY use these constructors — never instantiate
//! `SlotLatticeSearch` directly — to prevent accidental mismatch between
//! world shape and policy knobs.
//!
//! All policy knobs are set explicitly (no reliance on `SearchPolicyV1::default()`).

use sterling_search::policy::{DedupKeyV1, PruneVisitedPolicyV1, SearchPolicyV1};

use super::slot_lattice_search::{GoalProfile, SlotLatticeConfig, SlotLatticeSearch, TrapRule};

/// Test-side expectations for a regime.
///
/// Encodes the minimum observable thresholds that prove the regime's
/// stress axis was actually exercised (not just "search returned Ok").
#[derive(Debug, Clone)]
pub struct RegimeExpectations {
    /// Whether the first expansion should have `candidates_truncated == true`.
    pub expects_truncation: bool,
    /// Minimum `total_duplicates_suppressed` value.
    pub min_duplicates_suppressed: u64,
    /// Whether at least one `DeadEndReason::Exhaustive` should appear.
    pub expects_exhaustive_dead_end: bool,
    /// Minimum `frontier_high_water` value.
    pub min_frontier_high_water: u64,
    /// Whether the search should reach a goal.
    pub expects_goal_reached: bool,
}

/// A matched `(world, policy, expectations)` triple.
pub struct Regime {
    /// The slot lattice world instance.
    pub world: SlotLatticeSearch,
    /// The search policy tuned for this regime's stress axis.
    pub policy: SearchPolicyV1,
    /// Test-side expectations encoding the stress axis.
    pub expectations: RegimeExpectations,
}

/// Explicit policy constructor — all knobs set, no defaults drift.
fn explicit_policy(
    max_expansions: u64,
    max_frontier_size: u64,
    max_depth: u32,
    max_candidates_per_node: u64,
) -> SearchPolicyV1 {
    SearchPolicyV1 {
        max_expansions,
        max_frontier_size,
        max_depth,
        max_candidates_per_node,
        dedup_key: DedupKeyV1::IdentityOnly,
        prune_visited_policy: PruneVisitedPolicyV1::KeepVisited,
    }
}

/// **Candidate truncation** stress regime.
///
/// N=8, V=4 → root has 32 candidates. Policy caps at 5.
/// First expansion must have `candidates_truncated == true`.
#[must_use]
pub fn regime_truncation() -> Regime {
    let config = SlotLatticeConfig {
        active_slots: 8,
        values_per_slot: 4,
        trap_rule: TrapRule::None,
        goal_profile: GoalProfile::AllNonzero,
    };
    Regime {
        world: SlotLatticeSearch::new(config),
        policy: explicit_policy(
            100,  // max_expansions: enough to reach goal
            1000, // max_frontier_size: no frontier pressure
            100,  // max_depth: no depth limit
            5,    // max_candidates_per_node: forces truncation (32 → 5)
        ),
        expectations: RegimeExpectations {
            expects_truncation: true,
            min_duplicates_suppressed: 0,
            expects_exhaustive_dead_end: false,
            min_frontier_high_water: 1,
            expects_goal_reached: false, // may or may not reach goal with truncation
        },
    }
}

/// **Duplicate suppression** stress regime.
///
/// N=4, V=2 → many order-based duplicates from permuted slot assignments.
/// Goal=Never forces search to explore until budget/frontier exhaustion,
/// maximizing duplicate encounters.
#[must_use]
pub fn regime_duplicates() -> Regime {
    let config = SlotLatticeConfig {
        active_slots: 4,
        values_per_slot: 2,
        trap_rule: TrapRule::None,
        goal_profile: GoalProfile::Never,
    };
    Regime {
        world: SlotLatticeSearch::new(config),
        policy: explicit_policy(
            200,  // max_expansions: explore enough to see duplicates
            1000, // max_frontier_size: no frontier pressure
            100,  // max_depth: no depth limit
            100,  // max_candidates_per_node: no truncation
        ),
        expectations: RegimeExpectations {
            expects_truncation: false,
            min_duplicates_suppressed: 1,
            expects_exhaustive_dead_end: false,
            min_frontier_high_water: 2,
            expects_goal_reached: false,
        },
    }
}

/// **Exhaustive dead end** stress regime.
///
/// N=4, V=3, `trap=Slot0Eq(2)` → any state with `slot0==CONCEPT_VALUES[2]`
/// becomes trapped (enumerate returns empty). Goal excludes trap value
/// so trap branches are dead ends but non-trap branches can still reach goal.
#[must_use]
pub fn regime_exhaustive_dead_end() -> Regime {
    let config = SlotLatticeConfig {
        active_slots: 4,
        values_per_slot: 3,
        trap_rule: TrapRule::Slot0Eq(2), // slot0 == CONCEPT_VALUES[2] → trapped
        goal_profile: GoalProfile::AllNonzeroExceptSlot0Eq(2), // goal excludes trap value
    };
    Regime {
        world: SlotLatticeSearch::new(config),
        policy: explicit_policy(
            500,  // max_expansions: enough to explore trap + reach goal
            1000, // max_frontier_size: no frontier pressure
            100,  // max_depth: no depth limit
            100,  // max_candidates_per_node: no truncation
        ),
        expectations: RegimeExpectations {
            expects_truncation: false,
            min_duplicates_suppressed: 0,
            expects_exhaustive_dead_end: true,
            min_frontier_high_water: 2,
            expects_goal_reached: true, // non-trap branches can reach goal
        },
    }
}

/// **Budget-limited termination** stress regime.
///
/// N=6, V=3 → large state space. `max_expansions=3` forces early termination.
/// Goal=Never ensures termination is from budget, not goal.
#[must_use]
pub fn regime_budget_limited() -> Regime {
    let config = SlotLatticeConfig {
        active_slots: 6,
        values_per_slot: 3,
        trap_rule: TrapRule::None,
        goal_profile: GoalProfile::Never,
    };
    Regime {
        world: SlotLatticeSearch::new(config),
        policy: explicit_policy(
            3,    // max_expansions: forces budget termination
            1000, // max_frontier_size: no frontier pressure
            100,  // max_depth: no depth limit
            100,  // max_candidates_per_node: no truncation
        ),
        expectations: RegimeExpectations {
            expects_truncation: false,
            min_duplicates_suppressed: 0,
            expects_exhaustive_dead_end: false,
            min_frontier_high_water: 2,
            expects_goal_reached: false,
        },
    }
}

/// **Frontier pressure** stress regime.
///
/// N=6, V=3 → wide branching. `max_frontier_size=8` forces pruning.
/// Goal=Never ensures search explores until frontier/budget forces stop.
#[must_use]
pub fn regime_frontier_pressure() -> Regime {
    let config = SlotLatticeConfig {
        active_slots: 6,
        values_per_slot: 3,
        trap_rule: TrapRule::None,
        goal_profile: GoalProfile::Never,
    };
    Regime {
        world: SlotLatticeSearch::new(config),
        policy: explicit_policy(
            50,  // max_expansions: enough to generate frontier pressure
            8,   // max_frontier_size: forces frontier pruning
            100, // max_depth: no depth limit
            100, // max_candidates_per_node: no truncation
        ),
        expectations: RegimeExpectations {
            expects_truncation: false,
            min_duplicates_suppressed: 0,
            expects_exhaustive_dead_end: false,
            min_frontier_high_water: 8,
            expects_goal_reached: false,
        },
    }
}
