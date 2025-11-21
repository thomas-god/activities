# Maintaining feature availability and user experience across screen widths and platforms

_Date:_ 2025-11-21

## Context

Some features of _activities_, especially data-heavy ones, can be challenging to
build for smaller screen widths while maintaining a good user experience.

One solution could be to gate some features away from smaller screen widths or
specific platforms, but as a user, I personally find it frustrating when feature
availability is platform-dependent.

**Thus, we chose to make all features available on all screen widths and
platforms, at the cost of introducing extra interaction or navigation to
maintain a good user experience.**

One example would be the homepage's activity list:

- On larger screen widths, when a user selects an activity from the list, we
  display the activity's details next to the activity list using a two-column
  grid layout, minimizing navigation for a smoother experience.

- On smaller screen widths, on the other hand, when a user selects an activity
  from the list, we navigate to the activity's page, making sure its details are
  properly displayed using the screen's full width.

- In both cases, the activity's details displayed are the same, so that no
  information is hidden from the user.
