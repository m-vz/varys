# varys
Spying on smart speakers.

## macOS Caveats
### CMake
To build `whisper.cpp`, CMake is required. It can be installed using homebrew:
```shell
brew install cmake
```
Alternatively, it can be installed from https://cmake.org/download/. In this case, make sure `cmake` is in your `PATH`.

### Opus
If building Opus fails, it can be manually installed from https://opus-codec.org/ or with homebrew:
```shell
brew install opus
```

## Building
```sh
cargo build --release
```

## Running
Since varys is sniffing network packets, it needs to run as root:

```sh
sudo ./target/release/varys
```
