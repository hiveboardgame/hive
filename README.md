# Hive

TODO write something meaningful here

## Development

1. [Install Nix](https://nix.dev/install-nix.html) or just run:
```sh
curl -L https://nixos.org/nix/install | sh -s -- --daemon
```
2. Start the development shell
```sh
nix develop -c $SHELL
```
3. now start the server
```sh
migration run
server
```
4. (Optional) Setup direnv
5. Before committing code please run
```sh
format
```
