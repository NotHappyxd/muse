# Muse

This project was heavily inspired by snoowfall's [Lyse](https://github.com/snoowfall/lyse/) and Harman1307's [iris](https://github.com/Harman1307/iris).

Muse is a terminal-based lyric viewer written in Rust that dynamically themes itself based on album art.

Muse hooks into MPRIS (Linux-only) to detect the currently playing song, fetches synchronized lyrics from [lrclib.net](https://lrclib.net/), and uses k-means clustering to generate themes for the progress bar and lyric text.

## Features
* 🎵 MPRIS integration for detecting the currently playing track
* 📝 LRC synchronized lyric support
* 🎨 Dynamic theming based on album artwork
* ⚡ Lightweight(-ish) terminal user interface

## Installation
Clone the project, build it in release mode, and copy the binary into your bin folder.

```bash
git clone https://github.com/NotHappyXD/Muse
cd Muse
cargo build --release
cp target/release/muse ~/.local/bin/
```

## Usage

Run `muse` in the terminal.

Muse generates a configuration file at:

```text
~/.config/muse/config.toml
```

This file allows for basic customization.

Muse also caches lyric responses at:

```text
~/.cache/muse/
```

To clear the cache, delete this folder or remove individual song cache files.

## Acknowledgements
* The UI and core idea for this project were inspired by snoowfall's [Lyse](https://github.com/snoowfall/lyse/).
* Cluster scoring and color nudging were based on Harman1307's [iris](https://github.com/Harman1307/iris).
