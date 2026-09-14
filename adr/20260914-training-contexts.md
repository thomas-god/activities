# Implement tracking of additional training context

_Date:_ 2026-09-14

_Status:_ in progress/draft

## Context

Currently users can track individual training activities, training periods and training notes. While
already rich, this lacks important contexts like sleep, nutrition, well-being and more. We want to
add those contexts to enrich and facilitate the interpretation of one's training.

To avoid features creep and turning `activities` into an application that can track everything and
anything we define some criterion to help select relevant contexts to add:

- it must have an impact (positive or negative) on one's training,
- it must be easy to track daily (infrequent events can already be tracked using training notes),
- it must be quantitative (to facilitate comparison and correlation extraction).

While modern training devices produce a lot of different data (like HRV, sleep phases, etc.), we
chose to favor **subjective self-reported measures** over quantitative ones in order to limit this
feature's potential data creep and keep the UX simple. This trade-off is widely supported by current
research[^1].

## Implemented domains

All measures are at a daily interval.

- Hooper's index: 0-10 rank for fatigue, sleep, pain, stress and mood,
- Nutrition: total calories consumed (in kcal) and optional quantities of each macro-nutrients (in
  g),
- Hydration: amount of water consumed (in L) and optional alcohol consumption (in alcohol unit),
- Weight: total weight (in kg) and optional splits (fat, muscles, bone).

For each domain we tried to have a main, coarse, quantity that is easy to track, while allowing
optional finer quantities for users wanting to track more.

### Left-out domains

Quantitative measures :

- Sleep: total duration and per-phase splits,
- Resting heart rate, HRV, O2 saturation,
- Blood pressure, blood glucose.

---

[^1]:
    Saw, A. E., Main, L. C., & Gastin, P. B. (2016). Monitoring the athlete training response:
    subjective self-reported measures trump commonly used objective measures: a systematic review.
    British Journal of Sports Medicine, 50(5), 281–291. DOI:
    [10.1136/bjsports-2015-094758](https://doi.org/10.1136/bjsports-2015-094758).
