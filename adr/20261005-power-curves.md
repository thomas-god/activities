# Implementing power and pace curves

_Date:_ 2026-10-05

_Status:_ Draft

## Context

Power curves are a common tool to analyze your power distribution vs. holding time (e.g. "I'm able
to do 200W for 1min but 180W for 10min") since the relation between power and holding time is
non-linear and varies per athlete. It can help you identify which power zones to train for general
improvement or to train for a specific event and/or effort.

Currently the power curve is computed on the client when displaying an activity's details. This
provides only a single snapshot and would like to add the possibility to compare an activity's curve
to curves aggregating best performances from a given time span (last 12-weeks, all-time, for the
training period, etc.). Since the computation of a power curve is fairly compute intensive but
produce only a few values as the results we would like to move the computation to the server and
save it, so that retrieving the best power curve for a given period skips the computation part.

One open question is wether to add the weight dimension to the power curve computation (i.e.
normalized power curves), since having the same power values for different weights might represent
very different training points/states. The issue is that now a power curve is a function of
(activity, weight), and while an activity is an immutable entity for us, we would have to handle the
lifecycle of weight values to account for different situations:

- when initially importing your activities history and then you weight history,
- since an activity's weight can used weight values from up to 30 days before
