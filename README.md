# SMT5V Reverse Fusion Calculator

English | [简体中文](README.zh-CN.md)

A reverse fusion calculator for *Shin Megami Tensei V: Vengeance*. Find valid routes based on your target demon, required skills, and DLC settings, and change recipes for individual nodes.

Try it online: <https://megaten.rasp505.top/>

## Features

- Supports normal fusion, special fusion, and skills learned by leveling up.
- Provides exact route counts within the selected depth limit and displays the shallowest route by default; supports recipe changes, expanding and collapsing branches, and restoring the default route.
- Offers Simplified Chinese, Traditional Chinese, English, and Japanese interfaces, with cross-language name search.
- Includes light and dark themes, with an automatic mode that follows the system appearance.
- Adapts to desktops, tablets, and phones, with keyboard and touch support. Search settings, language, and appearance preferences are saved locally in the browser.

## Technology and Structure

Built with Rust, Leptos, and WebAssembly. Searches run in a Web Worker, with the complete route space represented as a compressed DAG. No backend is required; the application can be deployed as a static site.

- `crates/smt5-fusion-core`: Game data, fusion rules, route search, exact counting, and route replay validation.
- `crates/smt5-fusion-web`: Browser UI, Worker, localization, and interaction state.
- `tools/android-core-runner`: Native Android performance benchmarks for the core library.

## Local Development

Requires stable Rust. Run all commands from the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve --config crates/smt5-fusion-web/Trunk.toml
```

Open <http://127.0.0.1:8080/>. The development server rebuilds and reloads the page automatically, with caching disabled.

## Build and Deployment

```sh
trunk build --release --config crates/smt5-fusion-web/Trunk.toml --public-url /
```

## Tests

```sh
cargo test --locked --workspace --all-features
```

## License

[MIT](LICENSE)
