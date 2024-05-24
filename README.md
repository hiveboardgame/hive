# Hive

Hivegame is a free online hive game server focused mostly on realtime gameplay and ease of use.

It is written in rust nightly and relies on the [Leptos](https://leptos.dev/) framework. Pure hive logic is inside the engine workspace.

## Development

### Prerequisites

#### 1. Setup the Rust Toolchain
- Install the [Rust toolchain](https://www.rust-lang.org/tools/install).
- Set the default toolchain to nightly from the project root:
```sh
rustup override set nightly
```
- Alternatively set the default toolchain to nightly globally:
```sh 
rustup default nightly
```
- Add the WebAssembly target:
```sh
rustup target add wasm32-unknown-unknown
```

#### 2. Install Leptos
```sh
cargo install cargo-leptos
```

#### 3. Install and Setup PostgreSQL
- Follow the installation instructions for your OS ([example for Arch Linux](https://wiki.archlinux.org/title/PostgreSQL)).
- With PostgreSQL running, create a database `hive-local` and a user `hive-dev` as the owner:
```sh
sudo -u postgres createuser hive-dev && sudo -u postgres createdb -O hive-dev hive-local
```

### Manual Setup (Without Nix)

1. **Install Diesel CLI with PostgreSQL support**
```sh
cargo install diesel_cli --no-default-features --features postgres
```
2. **Install Leptos Formatter**
```sh
cargo install leptosfmt
```
3. **Run Database Migrations**
- From the project root, navigate to `db` and run migrations:
```sh
cd db && diesel migration run && cd ..
```
4. **Run the Project**
- The watch command will recompile your code when files change and serve it on localhost::3000, static changes inside the view! macro won't cause a recompile if running with the --hot-reload flag
```sh
cargo leptos watch --hot-reload
```
5. **Before Making a Pull Request please clippy and format your code**
```sh
cargo clippy --fix --all-features && leptosfmt apis -q && cargo fmt
```     

### Alternative to the manual setup use Nix

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
