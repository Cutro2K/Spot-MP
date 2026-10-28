# 🎵 Spot-MP

> **Spot-MP** es un reproductor de música interactivo para la terminal (TUI) desarrollado en **Rust**, diseñado para unificar tu biblioteca de música local y tu cuenta de **Spotify** en una experiencia fluida, rápida y navegable tanto por teclado como por mouse.

> **Spot-MP** aun se encuentra en fase de desarrollo, todavía no se presenta una versión funcional del sistema. Mi objetivo es tener una version funcional para enero/febrero de 2027.

---

## 💡 Idea del Proyecto

La idea principal de **Spot-MP** es resolver la fragmentación entre escuchar música almacenada localmente en tu equipo y reproducir música desde Spotify, todo sin salir de la terminal.

### Características Principales

- **Biblioteca Local:**
  - Lee, analiza y organiza los archivos de audio a partir de un directorio raíz configurable.
  - Reproducción directa, adición a cola o incorporación a playlists locales.

- **Integración con Spotify:**
  - Conexión opcional con tu cuenta de Spotify mediante autenticación OAuth2 con PKCE.
  - Búsqueda directa de canciones, álbumes y playlists en el catálogo de Spotify usando su API.
  - Importación y sincronización de playlists y álbumes guardados en tu perfil de Spotify.
  - Streaming integrado en la terminal mediante `librespot`.

- **Gestión Híbrida de Playlists:**
  - **Playlists Locales (Mixtas):** Listas creadas en tu base de datos local que pueden contener tanto archivos locales como pistas de Spotify.
  - **Playlists de Spotify:** Listas sincronizadas directamente con tu perfil de Spotify.

- **Reproductor General y Cola Inteligente:**
  - Control de reproducción unificado (play/pause, anterior, siguiente, slider de progreso).
  - Modos de reproducción: **Normal**, **Aleatorio (Random)** y **En Ciclo (Loop)**.

---

## ¿Dónde encontrar el diseño y análisis del proyecto?

Todo el proceso de ingeniería de software, análisis funcional y diseño de arquitectura se encuentra detalladamente documentado en la carpeta [`Engineering/`](./Engineering/):

```
Engineering/
├── Analisis/
│   ├── Idea.md                     # Especificación conceptual detallada y visión del producto
│   ├── Actores.md                  # Definición de actores del sistema (Usuario, Spotify API)
│   └── UseCases/                   # Casos de uso funcionales organizados por pantalla:
│       ├── LocalPage/              # Casos de uso de la vista de música local
│       ├── PlaylistPage/           # Casos de uso de gestión de playlists
│       ├── SpotifyPage/            # Casos de uso de la interacción con Spotify
│       ├── MusicPlayer/            # Casos de uso de los controles de audio
│       └── viewUseCases.md         # Resumen y mapa de casos de uso
└── Design/
    ├── Arquitecture/
    │   ├── FlowDiagram.png         # Diagrama de flujo global de la arquitectura
    │   ├── Diagrams.drawio         # Diagramas editables en Draw.io
    │   └── Backend/                # Diagramas de clases por capa:
    │       ├── audio_engine_class_diagram.png     # Motor de audio y backends
    │       ├── spotify_api_classes_diagram.png    # Cliente y autenticación Spotify
    │       ├── data_base_classes_diagram.png      # Controladores de persistencia
    │       └── Data_&_Storage_layer_diagram.png   # Capa de almacenamiento y archivos
    └── DataBase/
        └── Data_Base_Diagram.drawio.png           # Diagrama relacional de la base de datos (SQLite)
```

---

## 🛠️ Stack Tecnológico

- **Lenguaje:** [Rust](https://www.rust-lang.org/)
- **Interfaz en Terminal (TUI):** [`ratatui`](https://crates.io/crates/ratatui) y [`crossterm`](https://crates.io/crates/crossterm)
- **Motor de Audio:** [`rodio`](https://crates.io/crates/rodio) (reproducción local) y [`librespot`](https://crates.io/crates/librespot) (reproducción de Spotify)
- **Base de Datos & Metadatos:** [`sqlite`](https://crates.io/crates/sqlite), [`audiotags`](https://crates.io/crates/audiotags)
- **Autenticación:** OAuth2 PKCE para Spotify Web API

---

## Cómo Ejecutar el Proyecto

### 1. Prerrequisitos

- Tener instalado [Rust y Cargo](https://rustup.rs/) (edición 2024 / última versión estable recomendada).
- En sistemas Linux, asegurarte de contar con las librerías de desarrollo de audio y compilación (`ALSA` y `pkg-config`):

```bash
# En Debian / Ubuntu / derivados:
sudo apt update
sudo apt install build-essential pkg-config libasound2-dev libssl-dev

# En Fedora:
sudo dnf install pkgconf-pkg-config alsa-lib-devel openssl-devel

# En Arch Linux:
sudo pacman -S base-devel alsa-lib openssl
```

### 2. Clonar el repositorio

```bash
git clone https://github.com/TU_USUARIO/Spot-MP.git
cd Spot-MP
```

### 3. Compilar y Ejecutar

El código fuente de la aplicación se encuentra en el subdirectorio `spot-mp/`:

```bash
cd spot-mp
cargo run
```

### 4. Ejecutar pruebas y verificación

```bash
cargo check --manifest-path spot-mp/Cargo.toml
cargo test --manifest-path spot-mp/Cargo.toml
```

---

## Configuración de Spotify (Opcional)

Para habilitar las funciones de Spotify, necesitarás crear una aplicación en el [Spotify Developer Dashboard](https://developer.spotify.com/dashboard) para obtener tu `Client ID` y `Client Secret`, y configurar la URL de redirección correspondiente para el flujo de autenticación OAuth2 PKCE.

---
## 🤝 Contribuciones
¡El proyecto es de código abierto y cualquier sugerencia o idea es bienvenida!
Sin embargo, **actualmente el repositorio se encuentra en etapa de desarroll**. 
Por esta razón:
- **No se aceptarán *Pull Requests* de código por el momento**, con el fin de consolidar la arquitectura base y las funcionalidades esenciales sin generar conflictos ni inconsistencias.
- **¿Cómo puedes ayudar mientras tanto?** Si tienes ideas, sugerencias de diseño o mejoras, eres más que bienvenido a abrir un **[Issue](https://github.com/TU_USUARIO/Spot-MP/issues)** para debatirlas.
- **Apertura de contribuciones:** Una vez publicada la primera versión funcional estable, se abrirán oficialmente las contribuciones a la comunidad junto con una guía detallada (`CONTRIBUTING.md`).
¡Gracias por tu comprensión y por el interés en **Spot-MP**!