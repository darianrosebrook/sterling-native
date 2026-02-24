# Sterling Benchmark Report

**Generated:** 2026-02-06 12:20:58
**Sterling Version:** dev-53e7885d
**Platform:** Darwin 24.6.0
**Python Version:** 3.13.4
**Python Executable:** /Users/darianrosebrook/Desktop/Projects/sterling/.venv/bin/python
**Virtual Env:** /Users/darianrosebrook/Desktop/Projects/sterling/.venv

## Executive Summary

- **Total benchmarks:** 1
- **Passing (>= 80%):** 1
- **Pass rate:** 100.0%

## Results by Domain

### Rome Extended Navigation

**Status:** stable
**Description:** Navigation with varying hop distances (5/10/20) and step budgets (50/100/200)

| Provider/Model | Success | Rate | Latency | Time |
|----------------|---------|------|---------|------|
| sterling/structural12 | 360/360 | 100.0% | - | 121.6ms |

**Detailed Metrics (sterling/structural12):**

- **Evidence/Quality:**
  - Planned fixtures: 180
  - Planned attempts: 360
  - Planned tasks: 360
  - Attempted tasks: 360
  - Invalid tasks: 0
  - Invalid rate (planned): 0.000
  - Illegal actions (total): 0
  - Combos: 18
  - Episodes per combo: 10
- **Strategy Semantics:**
  - bfs: offline_shortest_path_executor
  - greedy_degree: online_greedy_degree
- **Fixture Identity:**
  - corridor: config_hash=9026250040e6d085 structure_hash=0b949b2e94d30d01 nodes=1001 edges=1000 max_oracle_distance=100
  - hub: config_hash=b2bdb67e8724a43f structure_hash=47db02144ca25c1c nodes=1000 edges=2651 max_oracle_distance=21
- **Termination Histogram (All Outcomes):**
  - solved: 360
- **Canary Reachability (Graph BFS):**
  - Corridor reachable: True | dist: 81
  - Hub reachable: True | dist: 21
- **Candidate Pools (Oracle Distances, for Fixture Health Only):**
  - corridor dist5: strict_window=30 relaxed_tail=960
  - corridor dist10: strict_window=30 relaxed_tail=910
  - corridor dist20: strict_window=30 relaxed_tail=810
  - hub dist5: strict_window=15 relaxed_tail=979
  - hub dist10: strict_window=15 relaxed_tail=954
  - hub dist20: strict_window=904 relaxed_tail=904
- **Success by Distance (Attempted Only):**
  - Corridor dist5: 100.0%
  - Corridor dist10: 100.0%
  - Corridor dist20: 100.0%
  - Hub dist5: 100.0%
  - Hub dist10: 100.0%
  - Hub dist20: 100.0%
- **Success by Strategy (Attempted Only):**
  - bfs: 100.0% (180/180)
  - greedy_degree: 100.0% (180/180)
- **Realized Distance + Budget Slack (Aggregated):**
  - Corridor dist5: attempted=60 invalid=0 success=100.0% realized_mean=6.3 realized_max=7 slack_min=43.0 slack_mean=110.4 relaxed_used=0
  - Corridor dist10: attempted=60 invalid=0 success=100.0% realized_mean=10.9 realized_max=12 slack_min=38.0 slack_mean=105.7 relaxed_used=0
  - Corridor dist20: attempted=60 invalid=0 success=100.0% realized_mean=21.1 realized_max=22 slack_min=28.0 slack_mean=95.5 relaxed_used=0
  - Hub dist5: attempted=60 invalid=0 success=100.0% realized_mean=5.9 realized_max=7 slack_min=43.0 slack_mean=110.8 relaxed_used=0
  - Hub dist10: attempted=60 invalid=0 success=100.0% realized_mean=11.0 realized_max=12 slack_min=38.0 slack_mean=105.7 relaxed_used=0
  - Hub dist20: attempted=60 invalid=0 success=100.0% realized_mean=21.0 realized_max=21 slack_min=29.0 slack_mean=95.7 relaxed_used=0

## Configuration

- **Domains:** rome_extended
- **Providers:** sterling
- **Tasks per domain:** 180

## Speed Metrics

- **Fastest:** 0.007ms
- **Slowest:** 0.053ms
- **Average:** 0.020ms
- **Weighted avg:** 0.020ms
- **P50:** 0.017ms
- **P90:** 0.036ms
- **Samples:** 360
