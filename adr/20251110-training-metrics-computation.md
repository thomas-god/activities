# Training metrics values computation and storage strategy

_Date:_ 2025-11-10

## Context

Training metrics (weekly distance, daily duration, etc.) are derived values
computed from user activities. These metrics can be displayed over various time
windows (1 month to 1+ year) and are frequently queried by the client.

The initial implementation persisted pre-computed metric values in the database
(`t_training_metrics_values` table). When activities were created, modified, or
deleted, the application would update the affected metric values across multiple
entry points:

- `create_activity`: update all the training metric affected by a new activity,
- `patch_activity`: recompute metrics affected by RPE/workout-type/nutrition
  based grouping
- `delete_activity`: remove activity contribution from metrics

This approach added significant complexity:

- Update logic scattered across domain entry points
- Risk of missing updates in new code paths
- Background async computation jobs
- Transaction coordination
- Potential for stale/inconsistent data

## Performance Analysis

To evaluate whether this complexity was justified, we benchmarked two
approaches : persisting training metric values vs. on-demand computation, using a
real dataset with several years of training data:

| Time Window  | Persisted (median) | On-Demand (median) | Difference | Ratio        |
| ------------ | ------------------ | ------------------ | ---------- | ------------ |
| **1 month**  | 3.67ms             | 4.23ms             | +0.56ms    | 1.15x slower |
| **3 months** | 3.63ms             | 4.85ms             | +1.22ms    | 1.34x slower |
| **6 months** | 3.36ms             | 6.55ms             | +3.19ms    | 1.95x slower |
| **1 year**   | 4.36ms             | 19.17ms            | +14.81ms   | 4.40x slower |

**Key findings:**

- Both approaches deliver excellent performance (< 20ms even for 1 year)
- On-demand computation scales linearly with activity count
- Persisted queries remain constant regardless of date range
- The performance difference (1-15ms) is imperceptible to users

## Decision

We will **remove metric value persistence** and compute metrics on-demand from
activities.

**Rationale:**

1. **Performance is excellent** - Even worst-case on-demand computation (19ms
   for 1 year) is well below acceptable latency thresholds (100-200ms)

2. **Significant complexity reduction** - Eliminates:
   - All `update_metrics_values()` calls across entry points
   - `remove_activity_from_metrics()` logic
   - `update_metrics_for_updated_activity()` complexity
   - Background async computation
   - `t_training_metrics_values` table and migrations
   - Risk of inconsistent/stale data

3. **Consistent with existing philosophy** - This mirrors our decision for
   timeseries storage (ADR 20250924): trade minimal latency for storage savings
   and code simplicity

4. **Easier evolution** - Metric calculation changes require only updating
   computation logic, no migrations or backfills

5. **Always fresh data** - Metrics reflect current activity state without cache
   invalidation concerns

## Implementation

The on-demand approach:

1. Store only metric definitions (what to compute, not the values)
2. Query activities in the requested date range
3. Compute metric values from activities on each request
4. Return results directly

Database schema changes:

- Keep: `t_training_metrics_definitions` (metric configurations)
- Remove: `t_training_metrics_values` (pre-computed values)

Service changes:

- Remove: `update_metrics_values()`, `remove_activity_from_metrics()`,
  `update_metrics_for_updated_activity()`
- Keep: `compute_training_metric_values()` (already implements on-demand logic)
- Remove: All calls to metric update methods from activity handlers

## Benefits and trade-offs

**Benefits:**

- Dramatically simpler codebase (remove ~500+ lines of update logic)
- No risk of stale/inconsistent metric values
- Easier to add new metrics or change calculations
- Reduced database storage and query complexity
- Consistent architecture with timeseries approach (ADR 20250924)

**Trade-offs:**

- 1-15ms additional latency per query (imperceptible to users)
- Slightly higher CPU usage for metric queries (negligible with typical activity
  volumes)

## Future Optimizations

If performance becomes a concern (unlikely given current measurements), we could
add a simple time-based cache:

- Cache computed values for 5-15 minutes
- Invalidate all cached metrics for a user on any activity change
- Much simpler than incremental persistence while providing near-instant
  repeated queries

This optimization is not implemented initially as the performance data shows
it's unnecessary.
