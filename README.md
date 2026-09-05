# next-ui

The reusable Bottles Next UI toolkit and its Iced desktop application.

## Package boundary

`next-ui` is one Cargo package with two Rust targets:

- library crate `next_ui`: the complete toolkit;
- binary crate `next-ui`: the application, enabled by the `application` feature.

Application state, Classic/Next workspaces, onboarding, and operation
orchestration are private binary modules. Toolkit users depend only on:

```rust
use next_ui::{theme, widget, Icon};
```

## Toolkit contract

The caller owns controlled data: values, selections, search results, status/log
data, progress, errors, dialog presence, and semantic messages. Widgets own
interaction mechanics: pointer/touch capture, focus and keyboard behavior,
popup disclosure, modal focus/capture, RowGroup disclosure, StatusBar
resizing/follow-tail, and animation.

`SelectorRow` expands in flow, while `Search` and `Popover` use anchored
overlays.
Use the normal label/icon button constructors unless arbitrary content requires
`Button::custom`.

Animated widgets use Iced's animation support directly; the toolkit exposes no
separate reduced-motion policy. Window chrome, close handling, resizing, and
global Tab traversal belong to the application shell.

The gallery is an external consumer of every public widget:

```bash
cargo run -p next-ui --example gallery --no-default-features
```

## Application lifecycle

The binary keeps the explicit `Workspace::{Classic, Next}` boundary and
separates semantic user `Action`s from task/subscription `Event`s:

```text
Booting → Onboarding/Running → Draining → Saving or ShuttingDown
                                      → Running or Exit
```

Feature actions are rejected while draining; close and focus navigation remain
global. Events continue to update the retained session until every active
operation emits a terminal result. Experience changes are committed only after
the complete `AppConfig` is atomically saved. A failed experience-switch save
restores the old session; a failed shutdown restores the retained session.

The private onboarding setup coordinator owns catalog refresh, required-runtime
downloads, cancellation, generations, stale-event rejection, and terminal phase
transitions. It uses the existing `bottles_core::Addons` operations; onboarding
UI only selects the experience/step and renders coordinator state.

## Features

- Default: `application` and `fvs`.
- `application`: enables app-only dependencies and the `next-ui` binary.
- `fvs`: enables `application` and `bottles-core/fvs`.
- `debug`: enables Iced hot reloading.

The toolkit builds without app/core dependencies using
`--no-default-features`.

## Accessibility

The public-release bar is complete keyboard operation, visible focus in light
and dark appearances, and at least 4.5:1 normal-text contrast. Reduced-motion
and screen-reader integration are not added beyond Iced's platform
capabilities.

## Verification

```bash
cargo fmt -p next-ui -- --check
cargo check -p next-ui --lib --no-default-features
cargo test -p next-ui --lib --no-default-features
cargo check -p next-ui --all-targets --no-default-features
cargo test -p next-ui --all-targets --no-default-features
cargo test -p next-ui --all-targets
cargo test -p next-ui --all-targets --all-features
cargo clippy -p next-ui --all-targets --no-default-features --no-deps -- -D warnings
cargo clippy -p next-ui --all-targets --all-features --no-deps -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc -p next-ui --lib --no-default-features --no-deps
cargo check -p next-ui --example gallery --no-default-features
```
