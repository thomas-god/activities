# Implementing duration curves (power and pace)

_Date:_ 2026-10-05

## Context

Power curves are a common tool to analyze your power distribution vs. sustain time (e.g. "I'm able
to do 200W for 1min but 180W for 10min"). Since the relation between power and holding time is
non-linear and varies per individual, it can help identify which power zones to train for overall
improvement or for a specific event or effort.

_**Definition**: a power curve is the highest rolling average values found for a set of durations
(usually from 5s to 60min+) from an activity's power timeseries. It can also be used for pace in
running, so a more generic term would be something like **duration curves**._

Currently the power curve is computed on the client when displaying an activity's details. This
provides only a single snapshot and we would like to add the possibility to compare an activity's
curve to a curve aggregating best performances from a given time span (e.g. last 12-weeks, all-time,
for a given training period, etc.). Since the computation of a power curve is fairly compute
intensive but produces only a few values, we would move the computation to the server and persist
its results, so that retrieving the best power curve for a given period skips the computation part.

_**Definition**: a best-power curve (defined over a given time span) takes for each duration value,
the best power value for that duration from all activities' duration curve within that time span._

## Key decisions and implementation details

#### Processing of existing activities

In the `t_duration_curves` table, each processed activity gets at least one row inserted, with null
values if the activity does not produce any duration curve. This sentinel row allows to detect
activities that have not been processed yet (since we assume an _already existing set of activities_
when introducing this feature) and a task from the activity service runs during the application
startup to process those.

#### Synchronization between the activity and training domains

The training service is responsible for computing best duration curves by aggregating curves from
activities. Since its more efficient to do this aggregation at the repository level in SQL we chose
to duplicate relevant duration curves (dropping null ones) on the training domain side. This is done
through an _outbox pattern_ using an outbox table in the activity repository and an intra-process
from the activity domain to the training domain.

> There is no explicit way to recompute an already computed duration curve. So if we update the
> algorithm or the target duration values we would have to clear the duration curves tables (likely
> via dedicated migrations) to force a recomputation of all activities' duration curves (including
> the activity repository's main and outbox duration curves table and the training repository
> duration curves table to keep the two domains in sync).

#### One duration curve per activity

In the `t_duration_curves` table we only store the curve type (pace or power), assuming there's a
one-to-one matching with the activity's sport (running -> pace, cycling -> power). This would no
longer hold if we were to introduce new curve types common to different sports (like heart rate) or
break the one-to-one matching (e.g. by using running power). In that case we would probably need to
add another column to carry the source activity's sport (or other relevant field) uses for later
grouping and/or filtering (e.g. "cycling power" metric would need to be able to filter out power
curve values from running activities).

#### Duration curves as training metric

Power and pace best-duration curves have been added as dedicated training metric sources. They
bypass most of a training metric definition usual fields (window, aggregate function, etc.) as they
have their own aggregation logic and produce stable bins (the set duration values, 5s, 10s, etc.)
regardless of the date range.

## Going further

A natural extension to this feature would be to have weight-normalized duration curves that take
into account the athlete's weight at the time of the activity (especially for climbing focused
users). Currently when switching a power curve display from W to W/kg on the client it uses the
current activity's weight to normalize both the activity's power curve and the 12-weeks best-power
curve, potentially skewing the latter.
