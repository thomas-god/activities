# Implement tracking of additional training context

_Date:_ 2026-09-14

_Status:_ in progress/draft

## Context

While users can currently track individual training activities, training periods and training notes,
one might lack important contexts like sleep, nutrition, well-being and more. We want to add those
contexts to enrich and facilitate the interpretation of one's training.

To avoid features creep and turning `activities` into an application that can track everything and
anything we define some criterion to help select relevant contexts to add:

- it must have an impact (positive or negative) on one's training (obviously),
- it must be easy to track daily (since infrequent events can already be tracked using training
  notes),
- it must be quantitative (to facilitate comparison and correlation extraction).

While modern training devices produce a lot of data (HRV, sleep phases, etc.), we chose to favor
**subjective self-reported measures** over quantitative ones in order to limit this feature's scope,
and keep the UX simple. This trade-off is widely supported by current research[^1].

## Implemented domains

All measures are recorded at a daily interval.

- Subjective feedback: using Hooper's index, 0-10 self-assessed rank for fatigue, sleep, pain,
  stress and mood,
- Nutrition: total calories consumed (in kcal) and optional quantities of each macro-nutrients (in
  g),
- Hydration: amount of water consumed (in L) and optional alcohol consumption (in alcohol unit),
- Weight: total weight (in kg) and optional splits (fat, muscles, bone).

_Subjective feedback_ cover the "how I'm feeling/responding to my training" part, _Nutrition_ and
_Hydration_ are things you can directly control, and _Weight_ allow to normalize performance (e.g.
FTP in W/kg).

For each domain we tried to have a main, coarse, quantity that is easy to track (e.g. 'Total
weight'), while allowing optional finer quantities for users wanting more detailed tracking (e.g.
'Muscle' and 'fat' quantities).

### Left-out domains

The following domains were left out as either too complicated to track, not bringing easy to
interpret training insight, or both:

- Sleep: total duration and per-phase splits,
- Resting heart rate, HRV, O2 saturation,
- Blood pressure, blood glucose.

## Impact on training metrics

Feedback, weight and nutrition are considered _regular_, i.e. they are defined each day, regardless
of if you track them or not (you eat, sleep and feel every day). In contrast activities are
_irregular_, in the sense that they do not necessarily happen every day.

This impact how training metrics using feedback, weight or nutrition:

- Definition: they necessarily have a granularity set (daily by default), while activity-based
  training metrics can have no granularity.
- Display: chart used to display them are continuous (line, areas) while activity-based training
  metrics use discontinuous ones (scatter plot, bar).

[^1]:
    Saw, A. E., Main, L. C., & Gastin, P. B. (2016). Monitoring the athlete training response:
    subjective self-reported measures trump commonly used objective measures: a systematic review.
    British Journal of Sports Medicine, 50(5), 281–291. DOI:
    [10.1136/bjsports-2015-094758](https://doi.org/10.1136/bjsports-2015-094758).
