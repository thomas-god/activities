# Activities storage

_Date:_ 2025-09-24

## Context

One key feature of _activities_ is to archive user training files, by writing
them on disk without any modification.

To support other features like activity viewing and training metric computation,
we also need to access the timeseries of an activity, that is the list of data
points (heart rate, speed, etc.) for the duration of the activity (usually at 1
second interval).

The first solution considered was to simply save those timeseries into an SQL
table, and retrieve them as needed. The issue with this approach was the size of
the resulting tables, nearly an order of magnitude more than the raw files.

Since the raw files are already optimized for size, we finally chose not to save
the timeseries in an SQL table, but instead reparse an activity file when we
need to access its timeseries. This works because:

- parsing an activity file is fast, usually a few ms, and thus does not degrade
  the overall user experience,
- most usecases do not require loading an activity timeseries so we do not pay
  this cost often.

One caveat is that "parsing an activity file is fast" is true mostly for `.fit`
files, as`.tcx` files are usually slower. But since `.tcx` are observed to be
fewer and for older activities, so this trade-off is considered acceptable.

## Benefits and trade-offs

**Benefits**: avoid duplicating data and optimize storage,

**Trade-offs**: the re-parsing of activity files add some latency, especially
for `.tcx` files.
