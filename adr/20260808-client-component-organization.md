# Client Component Organization Strategy

_Date:_ 2026-08-08

## Context

The frontend uses Svelte 5, Tailwind, and daisyUI as a component library. The
project initially followed atomic design (atoms / molecules / organisms) for
organizing components, but this broke down in practice:

- daisyUI already provides the atom layer (buttons, inputs, badges, etc.),
  leaving our `atoms/` folder essentially empty.
- The molecule/organism split was not based on any real reuse boundary in the
  app — it grouped components by relative complexity rather than by how they are
  actually used.
- Many components are purpose-built for a single domain (activity, training
  metric) and see little to no reuse outside that domain, yet were forced into
  the molecule/organism taxonomy anyway.
- Related components ended up scattered across folders (an organism far from the
  molecules it's composed of), hurting locality and making it harder to reason
  about a single feature's UI.

Atomic design assumes a large surface of generic, reusable components at every
level. In our case once the component library (daisyUI) owns the atom layer,
that assumption no longer holds, and the remaining tiers stop tracking anything
meaningful about the codebase.

## Decision

Replace the atomic design hierarchy with a two-layer structure:

1. **Shared components** (`ui/shared`): generic, reusable components with no
   domain or cross-domain knowledge — thin wrappers around daisyUI, generic
   composites (e.g. `SearchInput`, `DataTable`). A component lives here only
   once it is actually imported from more than one domain folder — not because
   it is _expected_ to be reusable.

2. **Domain-colocated components** (`ui/<sub-domain>/`): components,
   subcomponents, and any local logic specific to one feature/domain area,
   grouped together by what the app does rather than by abstraction level. This
   mirrors bounded-context boundaries: shared components are a shared kernel,
   and each feature folder is its own context.

**Promotion rule:** a component moves from a domain folder to `ui/shared` only
when a second, unrelated domain needs to import it. This replaces speculative
reuse (guessing something is an "atom" or "molecule" up front) with reuse based
on evidence.

### Public vs. private subcomponents within a feature

Within a feature folder, subcomponents that exist only to support one parent
component (and are not intended for reuse) are marked as private using an
`internal/` subfolder:

```
ui/training_metrics/
  TrainingMetricChart.svelte
  internal/
    TrainingMetricChartLine.svelte
    TrainingMetricChartStacked.svelte
```

This is enforced, not just conventional via an
[ESLint rule](/client/eslint-rules/no-cross-feature-internal-import.ts) forbids
importing anything under `**/internal/**` from outside its parent feature
folder.

A failure on a cross-boundary import into `internal/` is treated as a **signal
for promotion**: it means the subcomponent is no longer single-purpose and
should either be duplicated into the requesting feature (default, cheaper
choice) or promoted to `ui/shared` if the reuse is genuine and likely to recur.

## Consequences

**Positive**

- Folder structure reflects actual usage/reuse instead of a guessed taxonomy.
- Locality is preserved: a feature's components stay together, easier to reason
  about and refactor as a unit.
- The public/private boundary is mechanically enforced (lint + barrel exports)
  rather than relying on naming discipline alone.
- Cross-boundary imports become an actionable signal (promote vs. duplicate)
  rather than a silent violation.

**Negative / trade-offs**

- Two features may temporarily duplicate similar-but-not-identical components
  before a shared abstraction is justified. This is accepted as preferable to
  premature/incorrect abstraction.
- No compiler-level enforcement of privacy — the boundary depends on lint rules
  staying in place and being respected.
- Requires periodic auditing to catch components that have quietly become
  duplicated near-identically across features and should be promoted.

## Alternatives Considered

- **Continue with atomic design (atoms/molecules/organisms):** rejected —
  taxonomy doesn't track real reuse boundaries once daisyUI owns the atom layer;
  causes poor locality.
- **No marking, relying purely on folder scope:** rejected because it loses the
  explicit "promote or duplicate" signal that a private subcomponent provides
  when imported cross-feature.
