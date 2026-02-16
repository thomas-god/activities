# Computation of activity metrics derived from timeseries

_Date:_ 2026-02-16

## Context

Since [this ADR](20251110-training-metrics-computation.md) training metrics
(i.e. aggregation of activity metrics) are not persisted and are computed on
demand. This strategy works well for activity metrics that can directly be
extracted from the activity summary (distance, number of calories, etc.). But
for activity metrics that need to be derived from the activity's timeseries,
computing the training metric values requires to load all timeseries of the
activities in the training metric's scope. This is expensive as 1/ there could
be many activities depending of the date range and 2/ loading timeseries
requires re-parsing raw activity files.

In practice computing training metrics from timeseries introduces at least 1-2s
of delay when a user tries to request the training metric's values for a dozen
of weeks, which is too much, especially as the request is not cached.

## Decision

Since an activity's timeseries is immutable, we can considered metrics derived
from them as static. Thus it's possible to compute the values once and persist
them. Then computing training metrics for timeseries metrics would be as fast as
computing training metrics for other activity metrics.

The storage cost associated with this strategy is acceptable as we are not
persisting the whole timeseries, only a few floats per activity.

The main issues in implementing this strategy are :

- there is already an history of activities, so we don't want to introduce a
  clunky and hard to maintain process of computing metric values on existing
  activities,
- while timeseries are immutable, the way metrics are derived can evolve (new
  aggregation type or update to an existing one).

With those constraints in mind we decided to defer the computation of activity
metrics to when they are requested by the training service to the activity
service. The activity service then checks which activities have not yet been
processed for this particular activity metric, and compute and persist the new
metric values. Then all metric values are returned to the training service for
further grouping and processing.

## Benefits and trade-offs

**Benefits:**

- there is no dedicated process or logic for handling metrics of existing
  activities: the process of detecting activities needed to be process is the
  same for 1 new activity or 100+ existing ones.
- it's easy to add or update new metric type and aggregation function,
- an activity's metric value is computed only once for a given (metric,
  aggregation) and can be used by different training metrics.

**Trade-offs:**

- the first time a new (metric, aggregation) is used by a training metric, the
  full history must be processed (the issue described in the introduction). This
  is considered acceptable as this computation cost will be payed once and be
  amortized on subsequent uses.
- when storing the activity timeseries metrics, an extra column is used to
  indicate if the target (metric, aggregate) exists for the activity. This helps
  distinguish activities that have not yet be processed from those which have
  been processed but have no value, and thus avoid reparsing those each time.
