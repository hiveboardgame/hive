<p align="center">
  <img src="logo.png" alt="Hive Hydra Logo" width="300">
</p>

# Hive Hydra

Hive-hydra integrates any number of AIs with hivegame.com.

It communicates with hivegame.com using the Bot REST API and runs and communicates with multiple AIs (nokamute, for example) via UHP (Universal Hive Protocol) on standard I/O.

## Configuration

The application uses a YAML configuration file (default: `hive-hydra.yaml`) to define bot settings and API connections.

## Use

Usage: hive-hydra [OPTIONS]

Options:
  -c, --config <CONFIG>  Path to configuration file [default: hive-hydra.yaml]
  -h, --help             Print help
  -V, --version          Print version
