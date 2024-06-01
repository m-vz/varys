# varys
Spying on smart speakers.

## Installing
### Whisper Models
The whisper models can be downloaded from one of these locations:
- https://huggingface.co/ggerganov/whisper.cpp/tree/main
- https://ggml.ggerganov.com

More information about the models can be found [here](https://github.com/ggerganov/whisper.cpp/tree/master/models).

## Usage
The `varys` CLI contains comprehensive documentation about its usage. Use `varys help` for details on available commands and `varys help <COMMAND>` for the documentation of specific commands.

## Building
```shell
cargo build --release
```

## macOS Caveats
### CMake
To build `whisper.cpp`, CMake is required. It can be installed using homebrew:
```shell
brew install cmake
```
Alternatively, it can be installed from https://cmake.org/download/. In this case, make sure `cmake` is in your `PATH`.

### Launch Agent
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

### Opus
If building Opus fails, it can be manually installed from https://opus-codec.org/ or with homebrew:
```shell
brew install opus
```
