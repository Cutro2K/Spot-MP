# Spot-MP

> **Spot-MP** is an interactive terminal music player (TUI) built in **Rust**, designed to unify your local music library and your **Spotify** account into a smoother terminal expirience navigable via both keyboard and mouse.

> **Spot-MP** is still in active development, and a functional release is not available yet. My goal is to have a working version ready by January/February 2027.

---

## 💡 Project Idea

The main idea behind **Spot-MP** is to solve the fragmentation between listening to music stored locally on your machine and streaming music from Spotify, all without leaving the terminal.

### Key Features

- **Local Library:**
  - Reads, parses, and organizes audio files from a configurable root directory.
  - Direct playback, queuing, or adding tracks to local playlists.

- **Spotify Integration:**
  - Optional connection to your Spotify account via OAuth2 with PKCE authentication.
  - Direct search for tracks, albums, and playlists in the Spotify catalog using its Web API.
  - Import and synchronization of saved playlists and albums from your Spotify profile.
  - Integrated terminal streaming powered by `librespot`.

- **Hybrid Playlist Management:**
  - **Local (Mixed) Playlists:** Lists created in your local database that can contain both local audio files and Spotify tracks.
  - **Spotify Playlists:** Lists synchronized directly with your Spotify profile.

- **Unified Player & Smart Queue:**
  - Unified playback controls (play/pause, previous, next, progress slider).
  - Playback modes: **Normal**, **Random (Shuffle)**, and **Loop**.

---

## Where to Find the Project Design and Analysis

The entire software engineering process, functional analysis, and architecture design are documented in detail in the [`Engineering/`](./Engineering/) folder:

```
Engineering/
├── Analisis/
│   ├── Idea.md                     # Detailed conceptual specification and product vision
│   ├── Actores.md                  # System actors definition (User, Spotify API)
│   └── UseCases/                   # Functional use cases organized by screen:
│       ├── LocalPage/              # Local music view use cases
│       ├── PlaylistPage/           # Playlist management use cases
│       ├── SpotifyPage/            # Spotify interaction use cases
│       ├── MusicPlayer/            # Audio playback controls use cases
│       └── viewUseCases.md         # Summary and use case map
└── Design/
    ├── Arquitecture/
    │   ├── FlowDiagram.png         # Global architecture flow diagram
    │   ├── Diagrams.drawio         # Editable diagrams in Draw.io
    │   └── Backend/                # Class diagrams by layer:
    │       ├── audio_engine_class_diagram.png     # Audio engine and backends
    │       ├── spotify_api_classes_diagram.png    # Spotify client and authentication
    │       ├── data_base_classes_diagram.png      # Persistence controllers
    │       └── Data_&_Storage_layer_diagram.png   # Storage and file layer
    └── DataBase/
        └── Data_Base_Diagram.drawio.png           # Relational database diagram (SQLite)
```

---

## 🛠️ Tech Stack

- **Language:** [Rust](https://www.rust-lang.org/)
- **Terminal UI (TUI):** [`ratatui`](https://crates.io/crates/ratatui) and [`crossterm`](https://crates.io/crates/crossterm)
- **Audio Engine:** [`rodio`](https://crates.io/crates/rodio) (local playback) and [`librespot`](https://crates.io/crates/librespot) (Spotify playback)
- **Database & Metadata:** [`sqlite`](https://crates.io/crates/sqlite), [`audiotags`](https://crates.io/crates/audiotags)
- **Authentication:** OAuth2 PKCE for the Spotify Web API

---

## How to Run the Project

### 1. Prerequisites

- Install [Rust and Cargo](https://rustup.rs/) (2024 edition / latest stable version recommended).
- On Linux systems, make sure you have the audio and build development libraries installed (`ALSA` and `pkg-config`):

```bash
# On Debian / Ubuntu / derivatives:
sudo apt update
sudo apt install build-essential pkg-config libasound2-dev libssl-dev

# On Fedora:
sudo dnf install pkgconf-pkg-config alsa-lib-devel openssl-devel

# On Arch Linux:
sudo pacman -S base-devel alsa-lib openssl
```

### 2. Clone the repository

```bash
git clone https://github.com/YOUR_USERNAME/Spot-MP.git
cd Spot-MP
```

### 3. Build and Run

The application source code is located in the `spot-mp/` subdirectory:

```bash
cd spot-mp
cargo run
```

### 4. Run tests and checks

```bash
cargo check --manifest-path spot-mp/Cargo.toml
cargo test --manifest-path spot-mp/Cargo.toml
```

---

## Spotify Configuration (Optional)

To enable Spotify features, you will need to create an application in the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard) to obtain your `Client ID` and configure the corresponding Redirect URI (`http://127.0.0.1:8888/callback`) for the OAuth2 PKCE authentication flow (no `Client Secret` is required).

---

## 🤝 Contributing

This project is open source and any suggestions or ideas are welcome!
However, **the repository is currently in an active development stage**.
For this reason:
- **Code Pull Requests will not be accepted at this time**, in order to consolidate the core architecture and essential features without introducing conflicts or inconsistencies.
- **How can you help in the meantime?** If you have ideas, design suggestions, or improvements, feel free to open an **[Issue](https://github.com/YOUR_USERNAME/Spot-MP/issues)** to discuss them.
- **Opening up contributions:** Once the first stable functional version is released, community contributions will officially open along with a detailed guide (`CONTRIBUTING.md`).

Thank you for your understanding and your interest in **Spot-MP**!
