# Hive

TODO write something meaningful here

## Development
#### Prerequisites
- Setup the [rust toolchain](https://www.rust-lang.org/tools/install)
```rustup default stable```

- Install leptos
```cargo install cargo-leptos```

- Install and setup postgres ([example for arch](https://wiki.archlinux.org/title/PostgreSQL))

- With postgress running, create a database  `hive-local` and a user `hive-dev` as the owner
```
sudo -u postgres createuser hive-dev
sudo -u postgres createdb -O hive-dev hive-local
```

## 

1. [Install Nix](https://nix.dev/install-nix.html) or just run:
```sh
curl -L https://nixos.org/nix/install | sh -s -- --daemon
```
- Add these experimental features to your nix config (`/etc/nix/nix.conf`)
```
experimental-features = nix-command flakes
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

## License
This source code with the exception of the graphics is licensed under the GNU
Affero General Public License 3 license found in the LICENSE.md file in the
root directory of this project.
The official graphics for Hive are owned by [Gen42](https://gen42.com/) and may
not be used without prior written consent.
## Support
[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/T6T8XFTVA)
