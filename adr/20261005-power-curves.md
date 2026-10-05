# Implementing power and pace curves

_Date:_ 2026-10-05

_Status:_ Draft

## Context

Power curves are a common tool to analyze your power distribution vs. sustain time (e.g. "I'm able
to do 200W for 1min but 180W for 10min"). Since the relation between power and holding time is
non-linear and varies per individual, it can help identify which power zones to train specifically
for general improvement or for a specific event and/or effort.

_**Definition**: a power curve is highest rolling average values found for a set of durations
(usually from 5s to 60min) from an activity's power timeseries. Can also be used for pace in
running, so a more generic term would be something like **duration curves**._

Currently the power curve is computed on the client when displaying an activity's details. This
provides only a single snapshot and we would like to add the possibility to compare an activity's
curve to a curve aggregating best performances from a given time span (e.g. last 12-weeks, all-time,
for a given training period, etc.). Since the computation of a power curve is fairly compute
intensive but produces only a few values, we would like to move the computation to the server and
save it, so that retrieving the best power curve for a given period skips the computation part.

_**Definition**: a best-power curve (defined over a given timespan) takes for each duration value,
the best power curve value for that duration from all activities within that timespan._

### Steps

- Introduce duration curve for power and pace in the backend and compute them from an activity's
  timeseries.
  - Should handle history: i.e. already existing activities should have their power curve computed
    and persisted at some point.
- Persist duration curves per activity so that retrieving an activity with its details skip the
  duration curve computation.
- Update the client to no longer compute duration curves and use values from the backend instead.
- Update the training service to return a best-power curve for a given date range.

## Going further

A natural extension to this feature would be to have weight-normalized duration curves that take
into account the athlete's weight at the time of the activity (especially for climbing focused
users).

This might be a little trickier to implement as we can no longer just take the max value for each
duration, since each activity's weight might skew the values. Since the activity (contains duration
curve) and training (contains weight values) repositories are separated we cannot do this in a
single query to leverage the DB engine, but instead would have to load all values in memory and do
the aggregation in code, which might become expensive. This is compounded by the fact that
best-duration curves can be computed on arbitrary date ranges, so persisting results alone might not
help us that much. One way would be to persist power curves on the training domain side using an
outbound table mechanism, since `sync` primitives alone are not enough to guarantee durability.
