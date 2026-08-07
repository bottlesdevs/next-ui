# next-ui

The desktop GUI for Bottles Next, built with [`iced`](https://iced.rs) on top
of [`next-core`](../next-core).

- `components` — reusable UI components/widgets.
- `icons` — embedded icon assets (via `rust-embed`).
- `theme` — application theming.
- `operation` — UI-facing wrappers around long-running `bottles-core` operations.

Enable the `debug` feature to turn on `iced`'s hot-reloading during development.
