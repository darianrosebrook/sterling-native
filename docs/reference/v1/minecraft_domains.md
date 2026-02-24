# Minecraft Domains for Sterling

## Overview

Sterling's A* graph search with path-algebra learning maps naturally to many Minecraft autonomy problems. The conscious-bot project (separate TypeScript repo) connects to Sterling via WebSocket and delegates discrete planning problems as domain solves. This document catalogs the Minecraft problem domains, their graph formulations, and the domain handler contracts Sterling needs to support them.

The bot sends rules and state to Sterling at solve time — Sterling does not need Minecraft-specific knowledge baked in. Each domain is defined by how states are encoded, how edges are expanded, and what the goal condition is.

## Existing Domain: Crafting (Implemented)

### Problem

Given the bot's current inventory, find a sequence of craft/mine/smelt/place actions that produces a target item. The recipe tree is sent as rules at solve time, built dynamically from Mineflayer's `mcData`.

### Graph Formulation

| Element | Encoding |
|---------|----------|
| **Node** | Canonical inventory hash — sorted `"item:count"` pairs joined by `\|`. Example: `"oak_log:2\|oak_planks:4\|stick:4"` |
| **Edge** | A crafting rule applied to the inventory. Label = rule `action` string (e.g. `"craft:oak_planks"`) |
| **Start** | Current inventory state |
| **Goal** | Any inventory state where `goal[item] <= inventory[item]` for all goal items |
| **Cost** | `baseCost` per rule: mine=5.0, craft=1.0, smelt=3.0, place=1.5 |

### Solve Request Shape

```json
{
  "command": "solve",
  "domain": "minecraft",
  "inventory": { "oak_log": 0 },
  "goal": { "wooden_pickaxe": 1 },
  "nearbyBlocks": ["oak_log", "birch_log"],
  "rules": [
    {
      "action": "mine:oak_log",
      "actionType": "mine",
      "produces": [{ "name": "oak_log", "count": 1 }],
      "consumes": [],
      "requires": [],
      "needsTable": false,
      "needsFurnace": false,
      "baseCost": 5.0
    }
  ],
  "maxNodes": 5000,
  "useLearning": true
}
```

### Learning Benefit

Over repeated crafting episodes, Sterling learns which crafting chains converge fastest for common goals (e.g., wooden pickaxe from empty inventory always goes oak_log -> planks -> sticks -> table -> pickaxe). The path algebra increases `w_usage` on reliable edges and decays dead-end branches (e.g., trying to craft planks before having logs).

### Episode Reporting

After execution (success or failure), the bot sends:

```json
{
  "command": "report_episode",
  "domain": "minecraft",
  "goal": "wooden_pickaxe",
  "success": true,
  "stepsCompleted": 7
}
```

This feeds back into path algebra weight updates.

---

## Proposed Domains

### 1. Tool Progression

**Problem:** The bot must advance through material tiers (wood -> stone -> iron -> diamond). Each tier gates access to harder blocks and better tools.

**Graph Formulation:**

| Element | Encoding |
|---------|----------|
| **Node** | Capability state — set of unlocked capabilities. Example: `"has_wooden_pickaxe\|can_mine_stone"` |
| **Edge** | An upgrade action (craft tool, mine new material). Label = action string |
| **Start** | Current capability set (derived from inventory) |
| **Goal** | Target capability present (e.g. `"has_iron_pickaxe"`) |
| **Cost** | Composite: craft cost + gathering cost for prerequisites |

**Rules** are generated at solve time by inspecting which tools unlock which mining tiers:

- `wooden_pickaxe` -> can mine stone, coal ore
- `stone_pickaxe` -> can mine iron ore, lapis, gold ore
- `iron_pickaxe` -> can mine diamond ore, redstone, emerald
- `diamond_pickaxe` -> can mine obsidian

**Learning Benefit:** Learns the fastest upgrade path given typical world conditions (e.g., if stone is always nearby, skip extra wood gathering and go straight to stone pickaxe).

### 2. Smelting Chains

**Problem:** Produce smelted outputs (iron ingots, cooked food, glass, bricks) from raw materials + fuel + furnace.

**Graph Formulation:**

| Element | Encoding |
|---------|----------|
| **Node** | Inventory state (same encoding as crafting) |
| **Edge** | Acquire fuel, load furnace, wait, retrieve output |
| **Start** | Current inventory |
| **Goal** | Target smelted item in inventory |
| **Cost** | Time-weighted: smelting = 10s per item, mining fuel varies |

**Key Distinction from Crafting:** Smelting is time-dependent (each item takes 10 seconds) and requires fuel management. Rules include fuel options (coal, charcoal, wood planks, blaze rods) with different burn durations.

**Learning Benefit:** Learns optimal fuel choice. Charcoal from logs is renewable; coal requires mining. Over episodes, Sterling discovers which fuel source the bot can acquire fastest in its specific world.

### 3. Macro Navigation

**Problem:** Travel from current position to a distant target (biome, structure, waypoint). Block-level pathfinding is handled by Mineflayer; this is for high-level route selection.

**Graph Formulation:**

| Element | Encoding |
|---------|----------|
| **Node** | Known waypoint/landmark. ID = `"wp_{x}_{z}"` or `"biome_{name}_{x}_{z}"` |
| **Edge** | Travel between adjacent waypoints. Label = direction + terrain type |
| **Start** | Nearest waypoint to current position |
| **Goal** | Waypoint nearest to target |
| **Cost** | Estimated travel time (distance + terrain penalty + danger penalty) |

**Rules** are generated from the bot's explored map. Each known waypoint has edges to adjacent known waypoints with costs reflecting:
- Distance (Euclidean)
- Terrain difficulty (water crossings, mountains = higher cost)
- Danger (hostile mob density at night = higher cost)

**Learning Benefit:** Over exploration episodes, Sterling learns which routes are reliably safe and fast. A route through a ravine might be short but frequently leads to death; Sterling penalizes it via dead-end detection and usage decay.

### 4. Resource Acquisition

**Problem:** Acquire N units of a resource using the cheapest available method: mine directly, trade with villager, loot chest, craft from other materials.

**Graph Formulation:**

| Element | Encoding |
|---------|----------|
| **Node** | Resource state — `"iron_ingot:3\|coal:5"` |
| **Edge** | Acquisition method (mine, trade, loot, craft, smelt) |
| **Start** | Current resource counts |
| **Goal** | Target resource counts satisfied |
| **Cost** | Estimated time + risk per acquisition method |

**Rules** enumerate all known ways to acquire each resource type. Trade rules are generated from known villager offers; loot rules from known chest contents.

**Learning Benefit:** Learns which acquisition strategies succeed most often. If mining iron ore fails (no pickaxe, or ore is deep and dangerous), Sterling shifts weight toward trading or looting after repeated episodes.

### 5. Farm Layout

**Problem:** Design an efficient farm layout given available space and water sources.

**Graph Formulation:**

| Element | Encoding |
|---------|----------|
| **Node** | Partial farm layout — hash of placed blocks (water, farmland, crops) |
| **Edge** | Place water, till soil, plant crop at specific position |
| **Start** | Empty plot |
| **Goal** | All target crop positions planted + irrigated |
| **Cost** | Actions needed + water coverage efficiency |

**Key Constraint:** Water hydrates farmland within 4 blocks (Manhattan distance). A single water source block can irrigate an 9x9 area. Rules encode placement constraints.

**Learning Benefit:** Learns compact, high-yield layouts. The standard 9x9 farm with center water emerges as optimal after episodes exploring less efficient patterns.

### 6. Inventory Management

**Problem:** Decide which items to keep, store, or discard when inventory is full and the bot encounters a valuable item.

**Graph Formulation:**

| Element | Encoding |
|---------|----------|
| **Node** | Inventory configuration — slot assignments |
| **Edge** | Swap item, store in chest, discard item |
| **Start** | Current full inventory |
| **Goal** | Target item in inventory + minimum value lost |
| **Cost** | Value of discarded/stored items (context-dependent) |

**Rules** assign value scores to items based on current goals (iron ore is high-value when tool progression is active, low-value when the bot already has diamond tools).

**Learning Benefit:** Learns item valuation heuristics. Over episodes, Sterling discovers that keeping a stack of logs is always useful, but gravel is safe to discard.

### 7. Shelter Construction

**Problem:** Build a shelter structure by placing blocks in a valid order.

**Graph Formulation:**

| Element | Encoding |
|---------|----------|
| **Node** | Partial structure — set of placed blocks |
| **Edge** | Place block at position (x, y, z) with type |
| **Start** | Empty site (or existing partial build) |
| **Goal** | All target blocks placed |
| **Cost** | Repositioning time + block placement constraints |

**Key Constraints:** Blocks need support (can't float). Door placement requires adjacent walls. Roof requires wall support. Rules encode these physical constraints as preconditions.

**Learning Benefit:** Learns efficient build orders that minimize repositioning. Foundation-first sequences are faster than trying to build walls from the top.

### 8. Redstone Circuit Design

**Problem:** Place redstone components to achieve a target behavior (automatic door, item sorter, mob trap).

**Graph Formulation:**

| Element | Encoding |
|---------|----------|
| **Node** | Partial circuit — placed components + signal state |
| **Edge** | Place component (wire, torch, repeater, comparator, piston, observer) |
| **Start** | Empty circuit area |
| **Goal** | Target input/output behavior achieved |
| **Cost** | Component count + complexity |

**Learning Benefit:** Learns reliable circuit patterns. Redstone behavior is deterministic but non-obvious (signal strength, tick delays, quasi-connectivity). Sterling can discover working configurations through search and remember them.

### 9. Combat Encounter

**Problem:** Given detected hostiles, choose optimal tactical response.

**Graph Formulation:**

| Element | Encoding |
|---------|----------|
| **Node** | Combat state — `"health:15\|weapon:iron_sword\|mobs:zombie:2,skeleton:1\|terrain:open"` |
| **Edge** | Tactical action (attack, retreat, block entrance, eat, equip, use bow) |
| **Start** | Current combat state |
| **Goal** | All threats resolved OR bot at safe position |
| **Cost** | Health risk + time |

**Rules** encode tactical knowledge: skeletons require cover or shield; creepers require distance; zombies can be kited. Rules are sent at solve time based on detected mob types.

**Learning Benefit:** Learns which tactics work against which mob compositions. Discovers that retreating to a 1-wide corridor neutralizes multiple zombies better than fighting in the open.

### 10. Exploration Strategy

**Problem:** Choose which direction to explore next to maximize discovery.

**Graph Formulation:**

| Element | Encoding |
|---------|----------|
| **Node** | Map region — chunk-level granularity |
| **Edge** | Explore toward adjacent unexplored region |
| **Start** | Current region |
| **Goal** | Target discovery (specific biome, structure, or N new chunks) |
| **Cost** | Travel time + expected danger |

**Learning Benefit:** Novelty weighting (`w_novelty = 1/sqrt(1+visits)`) naturally drives toward unexplored regions. Over episodes, Sterling learns which exploration strategies yield the most discoveries per time (e.g., following rivers finds biome boundaries faster).

### 11. Task Scheduling

**Problem:** Given multiple pending goals with dependencies, choose which to work on next.

**Graph Formulation:**

| Element | Encoding |
|---------|----------|
| **Node** | Task portfolio state — set of completed/pending tasks |
| **Edge** | Work on task X (advances it toward completion) |
| **Start** | Current task states |
| **Goal** | All tasks completed |
| **Cost** | Estimated time + dependency wait time |

**Key Constraint:** Some tasks depend on others (can't smelt without furnace, can't mine iron without stone pickaxe). Rules encode these dependencies as preconditions.

**Learning Benefit:** Learns optimal task orderings. Discovers that gathering wood before anything else unblocks the most downstream tasks. Learns to batch-collect resources rather than context-switch.

### 12. Emergency Response

**Problem:** When multiple emergencies coincide (low health + hostile + dark + underwater), sequence responses optimally.

**Graph Formulation:**

| Element | Encoding |
|---------|----------|
| **Node** | Emergency state — active threats + resource availability |
| **Edge** | Emergency action (eat, retreat, place torch, surface, equip armor) |
| **Start** | Current emergency state |
| **Goal** | All threats resolved |
| **Cost** | Health risk during action + time |

**Rules** encode priority ordering: air (drowning) > health (bleeding out) > threat (hostile nearby) > light (mob spawning). But the optimal sequence depends on what the bot has in inventory and the specific combination of threats.

**Learning Benefit:** Pre-computes response plans for common emergency combinations. Learns that eating THEN retreating is better than retreating THEN eating (health buffer reduces damage during retreat).

---

## Extended Representational Patterns

Tiers 1-3 above cover known-state, goal-directed planning: the world is a mostly deterministic transition system and you do cheapest-path search with learning shaping edge ordering. To get broader Minecraft coverage, you can widen what kinds of "A becomes B" are encodable while staying within Sterling's state-graph framing. Each pattern below uses the same substrate — nodes are typed, hash-stable snapshots; edges are typed operators; learning prioritizes which legal edges to try first but does not invent transitions.

### Pattern 1: Epistemic Planning (Belief-State Graphs)

Instead of "what is true," nodes encode "what the bot believes is true" — a compact belief map plus what evidence supports it. Edges are information-gathering actions. The goal is reducing uncertainty enough to commit to an action plan.

**Minecraft instantiation: Structure/resource location**

| Element | Encoding |
|---------|----------|
| **Node** | Belief state — explored regions, sighting evidence (blaze particles, nether brick counts), structure likelihood per chunk, time since last probe |
| **Edge** | Probe action — move to vantage point, scan horizon, sample mob mix in biome, follow terrain features (rivers lead to biome borders, lava lakes indicate nether fortresses nearby) |
| **Start** | Prior belief (uniform uncertainty or seeded from world knowledge) |
| **Goal** | Target structure/resource located with confidence above threshold (e.g. P > 0.9) |
| **Cost** | Probe time + risk (nether probes cost health/gear) |

**State encoding:** `"explored:chunk_0_0,chunk_1_0|sightings:blaze:2@chunk_3_5|confidence:fortress:0.4@region_NE"`. Sorted and canonicalized.

**Edge examples:**
- `probe:high_ground` — climb to Y=120, scan 8-chunk radius, update sighting counts
- `probe:mob_sample` — enter biome for 30s, record mob spawns, update structure likelihood
- `probe:follow_river` — follow water downstream, biome transitions reveal boundaries
- `probe:eye_of_ender` — throw, track, triangulate stronghold (overworld-specific)

**Learning benefit:** Which probes produce the most information per unit risk/time, conditioned on biome and gear. Sterling learns that sampling mob mix in basalt deltas is high-information for fortress location, while random wandering in nether wastes is low-information.

**Goal condition:** Not a subset check — it's a threshold on the belief state's confidence field. The handler must evaluate `max(confidence[target]) >= threshold`.

---

### Pattern 2: Risk-Aware Planning (Cost Distributions, Not Point Costs)

Nodes remain discrete, but each edge carries a small outcome model: expected time, variance, and failure probability (death, gear loss, getting stuck). The goal becomes constrained optimization: minimize expected time subject to `P(death) < ε`, or maximize expected value with risk penalty.

**Minecraft instantiation: Obsidian acquisition**

| Element | Encoding |
|---------|----------|
| **Node** | State = gear loadout + food/potion supply + known lava pools + safe routes + time-of-day + nearby mob density estimate |
| **Edge** | Action with outcome distribution — mine obsidian at pool X (E[time]=30s, P[death]=0.08), barter for fire resistance first (E[time]=60s, P[death]=0.01), build temporary shelter (E[time]=45s, P[death]=0.0), travel at day vs night |
| **Start** | Current state |
| **Goal** | Target obsidian count met AND total P(death across plan) < ε |
| **Cost** | `expected_time + λ * risk_penalty` where λ scales with gear value at stake |

**State encoding:** `"gear:diamond_pick,iron_armor|food:8|fire_res:false|pools:lava_42_11_-30|time:day|mob_density:low"`.

**Edge cost model:** Each edge has `{ expected_time, time_variance, p_death, p_gear_loss }`. The path cost aggregates: `Σ expected_time + λ * (1 - Π(1 - p_death_i))`. The λ factor is set by the client based on gear value and respawn cost.

**Learning benefit:** Route and tactic priors that reduce tail risk, not only mean time. After dying at a particular lava pool, Sterling heavily penalizes that edge's `p_death` via dead-end detection. After surviving with fire resistance, Sterling promotes the "brew first" path.

**Handler difference:** `is_goal()` checks both resource satisfaction AND cumulative risk constraint. The heuristic must be risk-aware: `h = missing_items * min_time_per_item + risk_buffer`.

---

### Pattern 3: Invariant Maintenance (Non-Terminal Goals)

Many Minecraft problems are better expressed as "keep constraints true over time" rather than "reach a goal once." The state is a snapshot of invariant health (which are satisfied, which are drifting), and edges are repair/maintenance actions. The goal is all invariants restored.

**Minecraft instantiation: Base defense maintenance**

| Element | Encoding |
|---------|----------|
| **Node** | Base state — lit/unlit cells (light map hash), wall integrity (gap set), door states, mob incident history, current time, available repair materials |
| **Edge** | Maintenance action — place torches in unlit set, repair wall segment, build moat section, craft/place doors, replace broken fences, set spawn point |
| **Start** | Current base snapshot (some invariants violated) |
| **Goal** | All invariants satisfied: `light_coverage >= 0.95`, `wall_gaps == 0`, `doors_closed == true`, `mob_incidents_last_hour == 0` |
| **Cost** | Materials consumed + time + priority weight (unlit cells near beds are higher priority) |

**State encoding:** `"light:0.82|gaps:wall_N_3,wall_E_7|doors:open_S|incidents:2|time:night|materials:torch:12,cobble:30"`.

**Key difference from terminal goals:** The solve result is not a one-time plan. It's a repair schedule — the ordered set of actions that restores all invariants from the current degraded state. The bot re-solves periodically as invariants drift again.

**Learning benefit:** Which repairs prevent repeat incidents. Sterling learns that lighting the north wall prevents zombie spawns near the bed, while the south wall gap is low-priority because it faces a cliff. Over episodes, the path algebra learns causal structure of the base topology — which invariants are actually load-bearing for safety.

---

### Pattern 4: Network Design and Throughput (Infrastructure Graphs)

Once the bot builds rails, item transport (hoppers, water streams), nether ice roads, or storage systems, the design problem is naturally a graph over infrastructure topologies. The objective is throughput, latency, reliability, or expansion cost.

**Minecraft instantiation: Iron logistics optimization**

| Element | Encoding |
|---------|----------|
| **Node** | Infrastructure state — farm design parameters + item transport topology + storage routing rules + chunk-loading constraints |
| **Edge** | Infrastructure modification — add buffer chest, reroute hopper line, add water elevator, split item streams, add overflow protection, redesign for chunk boundaries |
| **Start** | Current infrastructure layout |
| **Goal** | Throughput target met (e.g. 64 iron/hour) with reliability constraint (no jams, no item loss) |
| **Cost** | Build cost (materials + time) + ongoing maintenance cost |

**State encoding:** Hash of `(source_positions, transport_edges, storage_nodes, chunk_boundaries)`. Transport edges have capacity and current utilization.

**Learning benefit:** Patterns that avoid jams and minimize build cost under chunk-loading realities. Sterling learns that hopper chains crossing chunk boundaries need buffering, and that water streams are cheaper than hopper lines for long distances.

---

### Pattern 5: Economy and Negotiation (Multi-Agent State Graphs)

Villagers, piglins, and (in multiplayer) humans create a domain where actions are social/economic and state includes reputations, trade tiers, workstation assignments, and constraints like "don't accidentally lock bad trades."

**Minecraft instantiation: Securing a Mending enchantment supply**

| Element | Encoding |
|---------|----------|
| **Node** | Villager roster state — `(profession, level, locked_trades[], curing_status, workstation_pos)` per villager + iron supply + time schedule + hero_of_village status |
| **Edge** | Social/economic action — assign workstation (convert profession), break workstation (reroll trades), cure zombie villager (discount), trade to level up, breed villagers, build iron farm for trade currency |
| **Start** | Current village state |
| **Goal** | At least one villager offers Mending at acceptable price |
| **Cost** | Resources spent + time + risk of losing existing good trades |

**State encoding:** `"v1:librarian:L2:locked[paper->emerald]|v2:unemployed|iron:32|cured:v1|time:day"`.

**Critical constraint:** Trade locking is irreversible. Once a villager levels up, their current trades are permanent. Rerolling requires breaking the workstation while the villager is still level 1 with unlocked trades. Rules must encode this irreversibility as a precondition guard.

**Learning benefit:** Which sequences reliably produce the target trade with minimal collateral damage. Sterling learns that curing zombie villagers first (for the discount) before leveling is cheaper than brute-force rerolling. Learns to avoid trading with a librarian to level 2 before confirming it has the desired enchantment at level 1.

---

### Pattern 6: Capability Composition (Typed Capability Algebra)

Generalize tool progression into full capability algebra. Nodes are capability sets ("can breathe underwater", "can survive nether lava", "can one-cycle end crystals", "can traverse 1000+ blocks efficiently"). Edges are "acquire capability" operations via crafting + enchanting + potions + infrastructure.

**Minecraft instantiation: Ocean monument raid preparation**

| Element | Encoding |
|---------|----------|
| **Node** | Capability set — enchantments held, potion recipes unlocked, conduit materials available, respiration/depth-strider gear status, food/health buffers, boat/trident status |
| **Edge** | Capability acquisition — brew water breathing, enchant helmet with respiration, build conduit, craft doors as air pockets, acquire trident from drowned, scout entry route |
| **Start** | Current capabilities |
| **Goal** | Capability set sufficient for monument raid: `{water_breathing, depth_strider, respiration, min_damage_output, food_buffer >= 20}` |
| **Cost** | Acquisition time + prerequisites |

**State encoding:** `"caps:water_breathing,depth_strider_2|gear:diamond_sword_sharp3,iron_helmet_resp2|potions:water_breathing_8m:3|food:24|conduit:false"`.

**Key difference from tool progression:** Capabilities compose non-linearly. Water breathing + depth strider + respiration together enable monument raids; any one alone is insufficient. The goal is a conjunction of capability predicates, not a single tier gate. Rules encode capability prerequisites (conduit requires heart_of_the_sea + nautilus_shells + prismarine).

**Learning benefit:** Discovers reliable minimal capability bundles — the smallest set that works consistently. These become macro-operators for future planning (the "monument prep loadout" becomes a reusable plan fragment).

---

### Pattern 7: Program Synthesis for Building (Plan-Level Search, Not Block-Level)

For shelters, farms, redstone, and large builds, searching over individual block placements explodes. A better representation is a DSL where edges add or refine "instructions" — parameterized templates rather than individual blocks. The executed block placements are the compilation of a higher-level plan.

**Minecraft instantiation: Shelter design and construction**

| Element | Encoding |
|---------|----------|
| **Node** | Partially specified build program — list of modules (foundation template, wall template, roof template, lighting pass, bed placement) + parameter bindings (dimensions, materials, door orientation) + terrain constraints |
| **Edge** | Program refinement — add module, set parameter, add safety pass, substitute material, compile to block list |
| **Start** | Empty program + site constraints (terrain slope, available materials, desired capacity) |
| **Goal** | Program compiles to valid block list satisfying shelter requirements (enclosed, lit, has bed, has door) |
| **Cost** | Material cost + estimated build time + complexity penalty |

**State encoding:** `"modules:foundation_5x5,walls_cobble_3h,roof_slab|params:door_S,bed_NE|materials:cobble:45,slab:25|compiled:false"`.

**Key advantage:** The search space is templates × parameters (hundreds) instead of blocks × positions (millions). Sterling explores which template combinations work for different terrains and material budgets. Compilation to actual block placements happens post-solve.

**Learning benefit:** Template selection and parameter priors. Sterling learns that "5x5 cobble box with slab roof" is the cheapest viable shelter, while "9x9 with interior lighting" is worth the extra cost for long-term bases. Fewer searches at block granularity while preserving auditability at the plan level.

---

### Pattern 8: Fault Diagnosis (Hypothesis Graphs)

Redstone circuits and complex farms often fail in ways that look like debugging. The state is "hypothesis about why it fails + current test evidence," and edges are tests/experiments. The goal is a confirmed hypothesis plus a repair plan.

**Minecraft instantiation: Item sorter debugging**

| Element | Encoding |
|---------|----------|
| **Node** | Diagnosis state — circuit schematic summary, observed symptoms (items backing up, wrong slot, signal dead), candidate fault set (hopper locked, comparator misconfigured, overflow, timing issue), test results so far |
| **Edge** | Diagnostic action — run test input pattern, instrument with temporary observer blocks, isolate module by breaking connection, swap component, re-test |
| **Start** | Symptom observation + full candidate set |
| **Goal** | Candidate set narrowed to single fault + repair applied + re-test passes |
| **Cost** | Test time + materials + risk of making it worse |

**State encoding:** `"symptoms:backup_slot_3,signal_dead_row_2|candidates:hopper_lock,comparator_mode,overflow|tests:input_pattern_A->fail,isolate_row_2->signal_restored"`.

**Learning benefit:** Which tests disambiguate fastest. Sterling learns that isolating modules is more informative than re-running inputs, and that comparator mode faults have a distinctive symptom pattern. Standard repair moves conditioned on symptom clusters become learned edges with high `w_usage`.

---

### Pattern 9: Exogenous Event Response (Contingent Policy Planning)

Some transitions are not chosen by the bot — night falls, rain starts, a creeper appears, hunger drops, a raid triggers. Represent these as observation/transition edges that change state without being "chosen." Sterling plans contingent policies around them.

**Minecraft instantiation: Day/night transition planning**

| Element | Encoding |
|---------|----------|
| **Node** | Situation state — current task plan + time-to-sunset + exposure level (how far from shelter) + gear/food status + nearby threat potential |
| **Edge (chosen)** | Bot action — shelter-up, light-up perimeter, return-to-base, switch-to-underground-mining, continue-current-task |
| **Edge (exogenous)** | World event — nightfall, rain_start, hostile_spawn, hunger_tick. These are modeled as forced transitions with known trigger conditions (time >= 13000 ticks, food_level < 6) |
| **Start** | Current situation |
| **Goal** | Safe continuation through the event — no death, minimal task disruption |
| **Cost** | Task disruption cost + safety risk |

**State encoding:** `"task:mining_iron|time_to_night:2min|exposure:high|shelter_dist:45|gear:iron|food:12|threats:none"`.

**Handler implementation:** The domain handler interleaves bot-chosen edges with forced exogenous edges when trigger conditions are met. This creates a game-tree-like structure where Sterling plans contingently: "if I keep mining, night will fall in 2 minutes and I'll be exposed; if I return now, I lose 2 minutes of mining but am safe."

**Learning benefit:** Robust contingency structures (policy fragments) that minimize disruption cost. Sterling learns the time threshold at which returning to base is worth the lost mining time, conditioned on gear quality and distance. After deaths from staying out too late, the "early return" edge gets reinforced.

---

## How These Patterns Stay Sterling-Shaped

All nine patterns reduce to the same substrate:

1. **A node** is a typed, hash-stable snapshot of the relevant state abstraction (belief map, risk-annotated inventory, invariant health vector, infrastructure topology, villager roster, capability set, build program, hypothesis set, situation context).

2. **An edge** is a typed operator that produces a new snapshot. The operator may be chosen by the bot or forced by the world (pattern 9), but it's always a discrete, well-defined transition.

3. **Learning** is allowed to prioritize which legal edges to try first (via path algebra: `w_usage`, `w_recency`, `w_novelty`), but it does not invent transitions. Search is the reasoning substrate; learning is advisory.

4. **Goal conditions** may be richer than subset checks — threshold predicates (pattern 1), constraint satisfaction (pattern 2), invariant conjunction (pattern 3), capability conjunction (pattern 6), compilation validity (pattern 7), hypothesis confirmation (pattern 8) — but they are still decidable Boolean functions on the state.

5. **Episode reporting** works uniformly: after execution, the client reports success/failure, and path algebra updates edge weights across the entire search graph for that domain.

---

## Domain Handler Contract

Each Minecraft domain requires a Sterling domain handler that implements:

```python
class MinecraftDomainHandler:
    """
    Domain handler for Minecraft graph-search problems.

    Unlike Rush Hour or Wikipedia where Sterling owns the graph structure,
    Minecraft domains receive the full rule set from the client at solve time.
    The handler's job is to:
    1. Parse the rules into an expandable graph
    2. Encode/decode inventory states as node IDs
    3. Apply rules to expand states (generate neighbors)
    4. Check goal conditions
    5. Compute heuristics (optional, domain-specific)
    """

    def solve(self, request: dict) -> AsyncGenerator[dict, None]:
        """
        Streaming solve. Yields discover/search_edge/solution/solution_path/complete
        messages following the standard Sterling WS protocol.
        """
        ...

    def expand_state(self, state: dict, rules: list[dict]) -> list[tuple[dict, dict]]:
        """
        Given a state and rule set, return all (next_state, edge_label) pairs
        where the rule's preconditions are satisfied by the current state.
        """
        ...

    def encode_state(self, state: dict) -> str:
        """
        Canonical hash of a state for node ID. Must be deterministic
        (sorted keys, consistent formatting).
        """
        ...

    def is_goal(self, state: dict, goal: dict) -> bool:
        """
        Check if state satisfies the goal condition.
        """
        ...

    def heuristic(self, state: dict, goal: dict, rules: list[dict]) -> float:
        """
        Admissible heuristic estimate. For crafting: count of missing items
        weighted by minimum rule cost to produce each.
        """
        ...
```

### Shared Infrastructure

All Minecraft domains share these patterns:

1. **Rules are client-provided** — Sterling doesn't need Minecraft item databases
2. **States are typed, hash-stable snapshots** — canonical string encoding (inventory hashes, belief maps, capability sets, infrastructure topologies, villager rosters, etc.)
3. **Goals are decidable predicates on the state** — may be:
   - Subset checks (crafting: `goal[item] <= inventory[item]`)
   - Threshold predicates (epistemic: `confidence[target] >= 0.9`)
   - Constraint satisfaction (risk-aware: `resource_met AND cumulative_p_death < ε`)
   - Conjunction checks (capability: `all(cap in state for cap in required_caps)`)
   - Invariant restoration (maintenance: `all(invariant_i satisfied)`)
   - Compilation validity (program synthesis: `program.compile() != null`)
   - Hypothesis confirmation (diagnosis: `|candidates| == 1 AND re-test passes`)
4. **Learning is edge-relative** — path algebra applies uniformly across all domain types
5. **Episode reporting** — client reports success/failure for weight updates
6. **Edges may be chosen or forced** — exogenous event patterns include world-event edges with trigger conditions; the handler interleaves forced transitions when conditions are met

### Configuration Hints

For Minecraft domains, recommended path algebra config:

```python
PathAlgebraConfig(
    alpha_usage=0.15,          # Moderate learning rate (crafting chains are short)
    alpha_recency=0.2,         # Moderate recency (recent successes matter)
    beta_base=1.0,             # Standard structural weight
    beta_usage=0.6,            # Moderate exploitation (crafting has few optimal paths)
    beta_recency=0.3,          # Mild recency boost
    beta_novelty=0.4,          # Moderate exploration (try alternative recipes)
    branch_decay_scratch=0.15, # Moderate decay on unchosen branches
    dead_end_penalty=0.3,      # Penalize impossible crafts (missing ingredients)
)
```

---

## Summary Table

### Tier 1-3: Known-State, Goal-Directed Planning

| Domain | Nodes | Edges | Goal | Learning Target |
|--------|-------|-------|------|-----------------|
| Crafting | Inventory hash | Craft/mine/smelt/place | Target item in inventory | Optimal crafting chains |
| Tool Progression | Capability set | Upgrade actions | Target tier unlocked | Fastest upgrade paths |
| Smelting Chains | Inventory hash | Acquire/smelt/retrieve | Smelted output in inventory | Optimal fuel choice |
| Macro Navigation | Waypoints | Travel between | Destination reached | Safe/fast route selection |
| Resource Acquisition | Resource counts | Acquire methods | Target counts met | Best acquisition strategy |
| Farm Layout | Partial layout | Place water/till/plant | Farm complete + irrigated | High-yield patterns |
| Inventory Mgmt | Slot assignments | Swap/store/discard | Target item + min loss | Item valuation heuristics |
| Shelter Construction | Partial structure | Place block | Structure complete | Efficient build orders |
| Redstone Circuits | Partial circuit | Place component | Target behavior | Working circuit patterns |
| Combat Encounter | Combat state | Tactical actions | Threats resolved | Per-mob-type tactics |
| Exploration Strategy | Map regions | Explore direction | Discovery target met | High-yield directions |
| Task Scheduling | Task portfolio | Work on task | All tasks done | Optimal task ordering |
| Emergency Response | Emergency state | Emergency actions | All threats resolved | Fast response sequences |

### Extended Patterns: Wider Representational Vocabulary

| # | Pattern | Node Type | Edge Type | Goal Type | Learning Target |
|---|---------|-----------|-----------|-----------|-----------------|
| 1 | Epistemic Planning | Belief map + evidence | Info-gathering probes | Confidence threshold | Best probe per biome/gear |
| 2 | Risk-Aware Planning | State + risk budget | Actions with outcome distributions | Resource target + P(death) < ε | Tail-risk reduction tactics |
| 3 | Invariant Maintenance | Invariant health vector | Repair/maintenance actions | All invariants restored | Causal structure of base safety |
| 4 | Network Design | Infrastructure topology | Add/reroute components | Throughput target + reliability | Jam-free patterns under chunk loading |
| 5 | Economy / Negotiation | Villager roster + economy | Social/economic actions | Target trade secured | Reliable trade sequences |
| 6 | Capability Composition | Typed capability set | Acquire-capability actions | Capability conjunction met | Minimal viable loadouts |
| 7 | Program Synthesis (Building) | Partial build program | Add module / set parameter | Valid compiled block list | Template + parameter priors |
| 8 | Fault Diagnosis | Hypothesis set + evidence | Diagnostic tests/swaps | Single fault confirmed + repaired | Fast-disambiguating test sequences |
| 9 | Exogenous Event Response | Situation context + timeline | Chosen + forced (world event) edges | Safe continuation through event | Disruption-minimizing contingencies |
