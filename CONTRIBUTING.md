# Contributing to hivegame.com

hivegame.com is built by a small group of volunteers in their spare time.
Every contribution helps: code, translations, bug reports and ideas.

## Talk to us

Join the [Hivegame Discord](https://discord.gg/jNTjr5vj9Z). It is the best
place to ask questions, discuss ideas and find out what is being worked on.

For anything bigger than a small fix, open an issue or ask on Discord before
you start, so we can agree on the approach before you spend time on it.

## Ways to help

- **Report a bug.** Open an [issue](https://github.com/hiveboardgame/hive/issues)
  with the steps to reproduce it, what you expected, what happened instead,
  and your browser and device.
- **Fix something.** Issues labelled
  [`good first issue`](https://github.com/hiveboardgame/hive/labels/good%20first%20issue)
  and [`help wanted`](https://github.com/hiveboardgame/hive/labels/help%20wanted)
  are good places to start.
- **Translate.** All UI text lives in `apis/locales/<language>/*.json`, with
  English (`en`) as the source. Missing keys fall back to English, so partial
  translations are welcome. You don't need Rust for this.
- **Connect a bot.** [Hive-hydra](hive-hydra/README.md) connects any engine
  that speaks the Universal Hive Protocol to hivegame.com.
- **Donate.** If you can't contribute code,
  [donations](https://hivegame.com/donate) keep the servers running.

## Repository layout

| Directory      | What it is                                                                 |
|----------------|----------------------------------------------------------------------------|
| `engine`       | Pure Hive rules: board, moves, history, hashing (package `hive`).          |
| `db`           | Postgres persistence with Diesel.                                          |
| `shared_types` | Types shared between the frontend and the backend.                         |
| `apis`         | The website: Leptos (SSR and hydration) on Actix, websockets, server functions. |
| `hive-hydra`   | Bot service that connects UHP engines.                                     |
| `busybee`      | Discord bot that delivers notifications.                                   |

## Development setup

Follow the [Development section of the README](README.md#development). Nix,
a manual setup and Docker are all supported.

## Before you open a pull request

1. Format the code: `leptosfmt apis -q && cargo fmt` (or `format` in the Nix
   shell).
2. Run `cargo clippy` and fix what it reports.
3. Run the tests for what you touched: `cargo test -p hive` for the engine,
   `cargo test -p db` for the database layer.
4. Keep pull requests focused on one change. Describe what changed and why,
   and add screenshots for UI changes (light and dark mode, and mobile if the
   layout changes).

## License

hivegame.com is licensed under the [GNU AGPL v3.0](LICENSE.md). By
contributing, you agree that your contributions are licensed under the same
terms.

The official Hive artwork in this repository belongs to
[Gen42 Games](https://www.gen42.com/) and is not covered by this license.
