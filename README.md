# amort-calc

A loan amortization calculator written in Rust with [Leptos](https://leptos.dev), compiled to WebAssembly.

**Live site: <https://amort-calc.pages.dev>**

## Features

- Live recalculation as you type (loan amount, APR, number of payments, start date)
- Summary panel: monthly payment, payoff date, total paid and total interest
- Amortization schedule with Monthly and Yearly views
- Shareable links: inputs are stored in the URL query (`?input=...`)
- App version (and build commit, when built in CI) shown in the footer

## Prerequisites

- Rust stable (pinned via `rust-toolchain.toml`, with rustfmt and clippy)
- The `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev): `cargo install trunk --locked`
- [just](https://github.com/casey/just): `cargo install just`

## Development

```sh
trunk serve            # dev server with live reload at http://127.0.0.1:8080
trunk build --release  # optimized build into dist/
```

## just commands

| Recipe | What it does |
|---|---|
| `just` | List recipes |
| `just fmt` | Format code |
| `just fmt-check` | Check formatting |
| `just clippy` | Clippy for wasm32 and host with `-D warnings` |
| `just test` | `cargo test --locked` |
| `just lint` | `fmt-check` + `clippy` |
| `just ci` / `just check` | lint + test + release build (same steps as CI) |
| `just serve` | `trunk serve` |
| `just build` | `trunk build --release --locked` |
| `just clean` | Remove `target/` and `dist/` |

## CI and deployment

GitHub Actions runs formatting, clippy, tests and a release build on every push and PR.
Pull requests get a Cloudflare Pages preview deployment (linked in a PR comment), and
merges to `main` deploy automatically to <https://amort-calc.pages.dev>.
