# varys

Spying on smart speakers.

## Installation

### Prerequisites for macOS on Apple Silicon
To use `whisper.cpp`, CMake is required. It can be installed using homebrew:
```sh
brew install cmake
```
Alternatively, it can be installed from https://cmake.org/download/. In this case, make sure `cmake` is in your `PATH`.

## Build

```sh
cargo build --release
```

## Starting

Since varys is sniffing network packets, it needs to run as root:

```sh
sudo ./target/release/varys
```
