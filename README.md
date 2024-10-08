# varys
A testbed to collect network captures, audio recordings, timestamps and a transcript of interactions with smart speakers. This data is used for large scale traffic fingerprinting on voice assistants. The ultimate goal hereby is to gain insight into security and privacy practices.

## Modules
This project is split into five modules:
- [varys](varys): The main executable
- [varys-analysis](varys-analysis): Experimental implementation of traffic fingerprinting and other analysis on data collected by varys
- [varys-audio](varys-audio): Recording audio and the TTS and STT systems
- [varys-database](varys-database): Abstraction of the database system where interactions are stored
- [varys-network](varys-network): Collection of network traffic, writing and parsing of `.pcap` files

## Installing
### Whisper Models
The whisper models can be downloaded from one of these locations:
- https://huggingface.co/ggerganov/whisper.cpp/tree/main
- https://ggml.ggerganov.com

More information about the models can be found [here](https://github.com/ggerganov/whisper.cpp/tree/master/models).

## Usage
The `varys` CLI contains comprehensive documentation about its usage. Use `varys help` for details on available commands and `varys help <COMMAND>` for the documentation of specific commands.

## Development
Dependencies for varys are kept in `flake.nix` that defines a Nix development shell. This means you don't need to install Rust or any other dependencies manually.

The dev shell has been tested on Linux, but should also work on macOS.

### 1. Installation
The Nix package manager is required to start the dev shell. Installation instructions can be found at https://nix.dev/install-nix and an introductory guide at https://nix.dev/tutorials/first-steps.

Additionally, Docker is required to start the development database.

### 2. Entering the Shell
To enter the development shell, run:
```sh
nix develop
```
By default, this starts a new Bash shell with the required dependencies installed.

A different shell (e.g. `zsh`) can be launched with `nix develop --command zsh`.

### 3. Database
Since `sqlx` checks SQL for correctness on the development database, we need to start it before building the project.

Start postgresql with:
```sh
docker compose up -d
```
Don't forget to stop the container when you're done working on varys. The container is configured to set up the development database in `data/db`. You can access it with `docker compose exec database psql -U postgres varys`.

Migrate the database with:
```sh
cd varys-database
sqlx migrate run
cd ..
```

### 4. Building
If you're working on varys and need to debug the build output, run:
```sh
cargo build
```

If you're deploying varys and need the best performance, run:
```sh
cargo build --release
```

### 5. Calibration
To calibrate the ambient noise before an experiment, place the microphone where the experiment will run and use
```sh
cargo run -- listen --calibrate
```

The resulting sensitivity can then be passed to varys using the `--sensitivity` parameter.

## macOS Launch Agent
To run varys as a daemon, move `local.varys.plist` to `~/Library/LaunchAgents` with permissions `644`.
In the file, replace `/path/to/varys` with the path to the varys executable, `/path/to/working/dir` with the path to where the data folder sits and `https://monitoring-url` with the url to the monitoring service.
Make sure the program arguments (voice, data path, etc.) are as desired.

To load and run the agent use:
```shell
launchctl bootstrap gui/`id -u` ~/Library/LaunchAgents/local.varys.plist
launchctl kickstart -kp gui/`id -u`/local.varys
```

To stop and unload the agent use:
```shell
launchctl kill SIGTERM gui/`id -u`/local.varys
launchctl bootout gui/`id -u` ~/Library/LaunchAgents/local.varys.plist
```
