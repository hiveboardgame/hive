# Hive

Hivegame is a free online hive game server focused mostly on realtime gameplay and ease of use.

It is written in rust nightly and relies on the [Leptos](https://leptos.dev/) framework. Pure hive logic is inside the engine workspace.

It also inclides [Hive-hydra](hive-hydra/README.md), an application to integrate Hivegame with any UHP (Universal Hive Protocol) AIs.

## Development

### Prerequisites (skip if using Nix)

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
- Edit `hba.conf` to `host   all   all  local 127.0.0.1/32 trust` (to allow passwordless login).
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
3. Init and Start the PostgreSQL DB
``` sh
pg-start
```

4. Now start the server
```sh
migration run
server
```
5. Start Hive-hydra to play with Bots

You need to setup bot users in Hive. See `hive-hydra/hive-hydra.yaml`

```sh
hive-hydra
```

5. Stop PostgreSQL
``` sh
pg-stop
```

4. (Optional) Setup direnv
5. Before committing code please run
```sh
format
```


### Docker Development

Ensure docker is installed on your machine. You can run the following command to start the server and database:

```sh
docker compose up --build -d
```

This command downloads and installs the required dependencies. The first time you run this command, it may take a few minutes to complete.

```sh
# to stop docker compose
docker compose down

# if you have already built the image, you can save some time by running without the build flag:
docker compose up -d
```

This will create a database, apply migrations and run the app on localhost:3000.

Once the containers are up, follow the app logs to watch cargo-leptos compile and see when the server is actually serving:

```sh
docker compose logs -f app
```
This shows the incremental build output and prints when the server binds to the port.

### Troubleshooting

If you get an error of the form
```
 it looks like the Rust project used to create this Wasm file was linked against
 version of wasm-bindgen that uses a different bindgen format than this binary:
 
   rust Wasm file schema version: 0.2.93
      this binary schema version: 0.2.95
 
```
That means that the Cargo.lock file was built with a different version of cargo-leptos than your current one.
You have two options to fix this. 
Either update cargo-leptos as the lock file was probably created by a newer version than you have or delete the lock file.

## License
This source code with the exception of the graphics is licensed under the GNU
Affero General Public License 3 license found in the LICENSE.md file in the
root directory of this project.
The official graphics for Hive are owned by [Gen42](https://gen42.com/) and may
not be used without prior written consent.
