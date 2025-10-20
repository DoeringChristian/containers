# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`containers` is a Rust CLI tool that simplifies container management by automating the build, lifecycle, and interaction with Docker/Podman containers. It provides a declarative workflow similar to `pixi.sh` or `nix dev shells`, where a Dockerfile defines the environment and the tool handles setup and execution automatically.

## Build and Development Commands

### Building
```bash
cargo build --release
```
The binary will be at `target/release/containers`.

### Testing
```bash
cargo test
```

### Running locally
```bash
cargo run -- [OPTIONS] [CONTAINER_NAME] [-- <COMMAND>...]
```

## Architecture

### Core Workflow (main.rs:63-94)
The application follows this lifecycle:
1. **Configuration** - Parse CLI args + env vars → unified `Config` struct
2. **Dockerfile Resolution** - Search upward from current dir to home/root
3. **Hash Generation** - Calculate SHA-256 hash of Dockerfile content during config creation
4. **Lockfile Loading** - Load/create lockfile in config to track Dockerfile state
5. **Image Management** - Build only when: forced update, no image exists, or Dockerfile changed
6. **Container Lifecycle** - Create, start, or exec into container based on current state

### Module Responsibilities

- **config.rs** - Merges CLI args with environment variables. Priority: CLI > env vars > defaults. Loads lockfile and calculates Dockerfile content hash. Uses first 12 characters of SHA-256 hash for container/image names (e.g., `a1b2c3d4e5f6:latest`).

- **engine.rs** (EngineType) - Defines Docker vs Podman enum with string parsing.

- **container.rs** (ContainerEngine) - Unified abstraction over Docker/Podman commands:
  - Detects NVIDIA GPU support automatically (nvidia-smi check)
  - Provides methods: `image_exists()`, `container_exists()`, `container_running()`, `build_image()`, `start_container()`, `exec_container()`, `create_and_run_container()`
  - Engine-specific GPU args: Docker uses `--gpus all`, Podman uses `--device nvidia.com/gpu=all`

- **dockerfile.rs** (DockerfileLocator) - Searches for Dockerfile by traversing upward from current directory to home directory, then checks home as fallback.

- **lockfile.rs** - Tracks Dockerfile state in `.containers.lock` (SHA-256 hash, mtime, size). Triggers rebuilds when Dockerfile changes. Stored alongside Dockerfile.

- **errors.rs** - Custom error types using `thiserror`: `BuildFailed`, `CommandFailed`.

### Key Design Patterns

**Intelligent Rebuild Detection**: Uses lockfile (`.containers.lock`) with SHA-256 hash comparison to avoid unnecessary rebuilds.

**Volume Mounting**: Mounts the Dockerfile's parent directory at the same absolute path in the container, preserving path structure for seamless development.

**Working Directory Preservation**: Tracks `current_dir` when entering container, sets it as working directory with `-w` flag in both `exec` and `run` commands.

**Hash-Based Naming**: Container and image names use the first 12 characters of the Dockerfile content's SHA-256 hash (e.g., `a1b2c3d4e5f6`). Names automatically change when Dockerfile content changes, creating fresh containers for modified environments.

## Environment Variables

- `CONTAINER_ENGINE` - Set to "docker" or "podman" (default: "podman")
- `DOCKERFILE` - Override default Dockerfile path
- `CONTAINER_NAME` - Override default container name

## Special Considerations

**Lockfile Location**: The `.containers.lock` file is stored in the same directory as the Dockerfile, not in the current working directory. Each Dockerfile has its own lockfile.

**Rebuild Triggers**: Container is removed and rebuilt when:
1. User passes `-u/--update` flag
2. Dockerfile content/mtime changes (detected via lockfile)
3. Image doesn't exist locally

**GPU Support**: NVIDIA GPU support is auto-detected at runtime by checking for working `nvidia-smi` command, then adds appropriate flags for the selected engine.
