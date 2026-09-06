# next-ui

The Bottles Next UI toolkit, desktop application, and component gallery, built
with Iced 0.14.

## Package boundary

`next-ui` remains one Cargo package with three targets:

- `next_ui`, the reusable library;
- `next-ui`, the desktop application;
- `gallery`, the executable catalog of toolkit behavior and appearance.

The supported library facade is deliberately small:

```rust
use next_ui::{theme, widget, Icon};
```

The facade contains no Bottles domain types. Bottles-specific composition stays
inside the application presentation modules. There is no speculative public
`component` layer; one should be introduced only when Classic and Next share a
concrete visual recipe that cannot remain a small function.

Window chrome belongs to the application shell and is not part of the public
toolkit.

## Architecture

```text
App
├── persistent theme
└── Session
    ├── persistent: core, config, canonical domain snapshots
    ├── lifecycle: Active, Draining, Saving, ShuttingDown
    ├── global overlay
    └── screen
        ├── Onboarding: setup coordinator and presentation
        └── Running
            ├── disposable features::State
            │   ├── bottles and program launch
            │   ├── library search
            │   ├── profiles and account linking
            │   ├── bottle settings
            │   └── snapshots (with `fvs`)
            └── disposable Presentation
                ├── Classic: routes, panels, tabs, dialogs, view composition
                └── Next: routes and view composition
```

Features are concrete state/update modules, not services or traits. Each feature
has one message path and may return a small typed output when a completed
workflow must affect presentation. Presentation-only actions, such as opening
profile settings, never round-trip through a feature.

The root keeps `Action` and `Event` separate only to enforce lifecycle safety.
User actions are accepted only while active. Task and subscription events keep
flowing while draining so pending operations can reach terminal state. Feature
events carry a run generation; events from a discarded experience are ignored.

Canonical bottle and profile snapshots are session-owned and continue updating
during onboarding, saving, and shutdown. Feature and presentation state are
recreated when the experience changes.

The onboarding coordinator owns catalog refresh, required-runtime downloads,
cancellation, generations, stale-event rejection, and terminal phase changes.
Its view only selects the experience/step and renders coordinator state.

## State ownership

State belongs to the lowest layer that has enough information and authority to
change it.

| State | Owner |
| --- | --- |
| Hover, press, keyboard focus | Widget tree |
| Popover disclosure, dismissal, popup focus | `Popover` |
| Expander disclosure and overlap eviction | `RowGroup`/standalone expander widget tree |
| Log disclosure, resize ratio, resize focus | `LogPanel` widget tree |
| Search query, loading/results/error data | Feature or screen |
| Selector value, switch value, active tab | Feature or screen |
| Dialog and side-panel presence | Presentation or root overlay |
| Drafts, pending operations, feature errors | `features::State` |
| Bottles, profiles, status and log contents | Canonical domain snapshots/features |
| Core, configuration, selected experience | Session/root |

Local widget state is discarded when a presentation is recreated. Domain and
workflow state is never hidden inside toolkit widgets.

Settings commands identify their target bottle. Their terminal completion does
not: completion must always reach the feature and clear its pending state even
if the bottle disappeared meanwhile.

## Toolkit contracts

- `Button` provides labeled, icon, disabled, loading, pill, and custom-content
  recipes. Icon-only buttons require an accessible label.
- Action rows are a single activation target and must not contain interactive
  descendants. Control rows are not themselves clickable.
- `ActionRow::new` accepts a message or `Option<Message>`;
  `ActionRow::progress` represents non-interactive progress.
- `SelectorRow` is an in-flow connected selector. `Popover` and `Search` use the
  shared private anchored overlay.
- `ExpanderRow` accepts ordinary `Into<Element>` children. `RowGroup` owns the
  connected header/panel drawing, column footprints, and eviction of overlapping
  panels while allowing non-overlapping panels to remain open.
- Search data is caller-owned and represented by `Hidden`, `Loading`, `Empty`,
  `Error`, or `Results`. The optional footer is a supported action.
- `StatusBar` is domain-neutral. `LogPanel` is the Bottles convenience recipe
  that supplies architecture, runner, status, and optional log contents while
  retaining local disclosure and resizing.
- Dialog presence is controlled by the caller. `WindowModal` makes the base
  inert and owns modal focus, Tab traversal, Escape, and outside dismissal.
- Animations use Iced's native animation support. Canvas is retained only for
  actual geometry: the dashed `ActionTile`, animated `Switcher` knob, and
  `ProgressRing`.

## Theme

`theme::dark()` and `theme::light()` construct custom Iced themes using Iced's
extended palette. `theme::colors()` exposes the toolkit's semantic names from
that palette so widget recipes do not reinterpret palette slots independently.

The original light and dark muted colors are retained. There is no automated
contrast assertion or claim that every muted-text pairing meets a fixed ratio.
Keyboard operation, meaningful icon labels, and visible focus remain required
interaction behavior.

## Gallery coverage

The gallery is an external consumer of the complete public facade.

| Area | Covered concepts |
| --- | --- |
| Typography | `TextExt`, `Title` subtitle/status |
| Actions | `Button` variants, `ActionTile` enabled/disabled |
| Cards | `Card`, `ArtworkCard`, `CardAction`, every `InfoCardKind` |
| Navigation | `Tabs`, `Tab`, `Popover`, `PopoverItem` |
| Search | hidden, loading, empty/results, error, result actions, footer |
| Rows | `ListRow`, `ActionRow`, `InfoRow`, `TextRow`, `SelectorRow`, `SwitcherRow`, `CycleRow`, `PickerRow` |
| Disclosure | standalone `ExpanderRow`, connected `RowGroup`, overlapping and non-overlapping matrices |
| Status | generic `StatusBar`, `LogPanel`, danger and no-log states |
| Standalone controls | `Switcher`, `ProgressRing` |
| Shell | `HeaderBar`, `Dialog`, `WindowModal` |

Run it with:

```bash
cargo run -p next-ui --example gallery --no-default-features
```

## Experience switching and shutdown

Switching experiences:

1. Confirms the target.
2. Rejects new UI actions.
3. Cancels disposable workflows that support cancellation.
4. Continues accepting terminal events until every operation drains.
5. Atomically saves the target experience.
6. Recreates feature and presentation state over the same core and canonical
   snapshots.
7. Restores the source experience if saving fails.

A close request during draining or saving converts the transition into shutdown.
A failed shutdown restores the retained session. Window actions are handled
exhaustively by the root shell.

## Features

- Default: `fvs`.
- `fvs`: enables `bottles-core/fvs`.
- `debug`: enables Iced hot reloading.

`--no-default-features` builds the complete package without FVS support.

## Custom widget inventory

The toolkit contains eight private Iced `Widget` implementations and one shared
private `Overlay` implementation. Public callers interact only with recipes.

| Private implementation | Single responsibility |
| --- | --- |
| `FocusableAction` | Shared pointer, touch, Enter/Space, and focus activation |
| `RowGroupCore` | Connected grid layout and overlap-aware disclosure |
| `Selector` | Connected in-flow selector disclosure and keyboard interaction |
| `SearchWidget` | Search-input focus and anchored result disclosure |
| `PopoverWidget` | Trigger disclosure, dismissal, and popup focus |
| `LogWidget` | Local log disclosure and pointer/keyboard resizing |
| `ModalBase` | Preserve/draw the base while making it inert |
| `ModalScope` | Capture modal input, focus traversal, and dismissal |
| `PopupCore` (`Overlay`) | Shared anchored placement and outside dismissal |

Everything else is native Iced composition, styling, or a canvas drawing
program.

## Size audit

Toolkit size is measured as physical lines in `src/lib.rs`, `src/icons.rs`,
`src/theme.rs`, and `src/widgets/*.rs`, including inline tests.

- Before the architecture rewrite: 9,445 lines.
- After the architecture rewrite: 7,684 lines (1,761 fewer, an 18.6% reduction).

The remaining total is above the aspirational 6,000–7,000 range because the
required connected multi-column expanders, original in-flow selector, accessible
anchored overlays/modal behavior, animations, and resizable log panel are
implemented locally. These are deliberate cohesive responsibilities rather than
public interaction frameworks.

## Verification

Every implementation commit must compile independently. Phase and final gates:

```bash
cargo fmt -p next-ui -- --check
cargo check -p next-ui --all-targets
cargo test -p next-ui --all-targets
cargo test -p next-ui --all-targets --no-default-features
cargo test -p next-ui --all-targets --all-features
cargo clippy -p next-ui --all-targets --all-features --no-deps -- -D warnings
cargo doc -p next-ui --lib --no-deps
git diff --check
```
