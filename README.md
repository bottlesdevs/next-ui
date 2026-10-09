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
inside the application screens and shared `views` module.

Window chrome belongs to the application shell and is not part of the public
toolkit.

## Architecture

```text
App: system theme, Loading / Failed / Ready, selected screen
└── Context
    ├── core: shared Bottles services and registered plugin providers
    ├── settings: selected experience, persisted in core's config_dir/ui.toml
    ├── store: latest bottles, programs (fvs), library, and addon snapshots
    ├── jobs: tracked operations, progress, and cancellation tokens
    └── notifications: dismissible cards with copy actions
```

Boot opens core, loads installed plugins, registers their account and library
providers, and loads UI settings. `Store::watch` starts `Task::run` streams once
at boot. Bottles and standalone program snapshots are sorted by name.

Every screen receives the same `Context`. Screens call core mutations through
`ctx.jobs.start`; shared fetch and library-launch helpers live on the context.
Jobs stream progress, discard successful return values, and remove themselves
on completion. Effects arrive through the core watch streams. Non-cancelled
failures become notifications, alongside immediate core errors and settings
save failures.

Onboarding derives required Runner/WineBridge and supported optional
UMU/DXVK/VKD3D/LatencyFlex rows from the addon snapshots and jobs. Classic owns
its tabs, selected bottle, creation draft, and rename input. Next owns a search
query and selected standalone program, and requires `fvs`. Both experiences
use the application-owned animated split layout. `views` contains shared
pages, header buttons, and grids.

The toolkit remains free of domain types. Its public `Notification` card
provides kind styling and dismiss/copy actions; the application owns the
notification stack and its scrollable bottom-right layer.

## State ownership

| State | Owner |
| --- | --- |
| Hover, press, keyboard focus | Widget tree |
| Popover disclosure, dismissal, popup focus | `Popover` |
| Expander disclosure and overlap eviction | `RowGroup`/standalone expander widget tree |
| Log disclosure, resize ratio, resize focus | `LogPanel` widget tree |
| Search query, tabs, selection, drafts | Screen |
| Bottles, standalone programs, library, addon state | Context store |
| In-flight operations, progress, cancellation and shutdown drain | Context jobs |
| Notifications, core, settings | Context |
| Theme and startup state | App |

Local widget and screen state is discarded when an experience is recreated.
Snapshots, jobs, and notifications survive experience switches. The shared
registry receives every terminal job event independently of the initiating
screen.

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

The gallery is an external consumer of the public facade.

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

Selecting an experience saves its settings in the background and swaps only
screen state. Core, watches, snapshots, jobs, and notifications stay in the
context. Settings writes are jobs queued in selection order. Next is unavailable
without `fvs`; an existing Next preference falls back to Classic in that build.

Closing displays “Finishing operations…”, cancels registered work, and waits for
the `TaskTracker` to drain before calling `core.shutdown()` and exiting. Pending
settings jobs also finish before exit. Jobs are registered before their first
poll, so close cannot miss newly queued work. A close request during startup
waits for boot and shuts down the opened core. Chrome and Tab focus traversal
remain in the root shell.

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
- After the toolkit architecture rewrite: 7,684 lines (1,761 fewer, an 18.6% reduction).
- With the public Notification card: 7,765 lines.

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
