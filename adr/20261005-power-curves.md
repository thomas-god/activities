# Implementing duration curves (power and pace)

_Date:_ 2026-10-05

_Status:_ Draft

## Context

Power curves are a common tool to analyze your power distribution vs. sustain time (e.g. "I'm able
to do 200W for 1min but 180W for 10min"). Since the relation between power and holding time is
non-linear and varies per individual, it can help identify which power zones to train specifically
for general improvement or for a specific event and/or effort.

_**Definition**: a power curve is the highest rolling average values found for a set of durations
(usually from 5s to 60min+) from an activity's power timeseries. It can also be used for pace in
running, so a more generic term would be something like **duration curves**._

Currently the power curve is computed on the client when displaying an activity's details. This
provides only a single snapshot and we would like to add the possibility to compare an activity's
curve to a curve aggregating best performances from a given time span (e.g. last 12-weeks, all-time,
for a given training period, etc.). Since the computation of a power curve is fairly compute
intensive but produces only a few values, we would move the computation to the server and persist
it, so that retrieving the best power curve for a given period skips the computation part.

_**Definition**: a best-power curve (defined over a given time span) takes for each duration value,
the best power value for that duration from all activities within that time span._

## Key implementation details and decisions

In the `t_duration_curves` table, each processed activity gets at least one row inserted, with null
values if the activity does not produce any duration curve. This allows to detect activities that
have not been processed yet (since we assume an _already existing set of activities_ when
introducing this feature) and a task from the activity service runs during the application startup
to process those.

The training service is responsible for computing best duration curves by aggregating curve from
activities. Since its more efficient to do this aggregation at the repository level in SQL we chose
to duplicate relevant duration curves (dropping null ones) on the training domain side. This is done
through an _outbox table + intra-process notification pattern_ from the activity domain to the
training domain.

## Going further

A natural extension to this feature would be to have weight-normalized duration curves that take
into account the athlete's weight at the time of the activity (especially for climbing focused
users).
