<p align="center">
  <img src="logo.png" alt="Hive Hydra Logo" width="300">
</p>

# Hive Hydra

Hive-hydra integrates any number of AIs with hivegame.com.

It keeps one WebSocket per bot open to hivegame.com (see [`BOT_WEBSOCKET_API.md`](../BOT_WEBSOCKET_API.md)), so the server pushes each opponent move and direct challenge as it happens. Fetching a bot's token is the only HTTP request. For every turn it runs an AI (nokamute, for example) and talks to it via UHP (Universal Hive Protocol) on standard I/O.

The WebSocket address is derived from `base_url`: `http://host` becomes `ws://host/ws/?format=json`, `https://host` becomes `wss://host/ws/?format=json`.

## Configuration

The application uses a YAML configuration file (default: `hive-hydra.yaml`) to define bot settings and API connections.

## Use

Usage: hive-hydra [OPTIONS]

Options:
  -c, --config <CONFIG>  Path to configuration file [default: hive-hydra.yaml]
  -h, --help             Print help
  -V, --version          Print version
