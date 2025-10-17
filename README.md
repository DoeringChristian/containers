# containers - Simplified Container Management

`containers` is a powerful command-line utility designed to streamline your
container workflow. It automates the tedious process of building, managing, and
interacting with container environments, whether you prefer Docker or Podman.
This tool is ideal for developers who want to focus on their code, not on
container commands. The experience is similar to using tools like `pixi.sh` or
`nix dev shells`, where a declarative file defines the development environment,
and the tool handles the setup and execution.

The core philosophy of `containers` is to provide a seamless and intuitive
experience by intelligently handling the container lifecycle. It automatically
detects your `Dockerfile`, builds images when necessary, and gets you into a
running container with a single command.

## Key Features

- **Automatic Dockerfile Detection**: Scans the current directory and parent
  directories to find the relevant `Dockerfile`.
- **Intelligent Image Building**: Builds container images only when needed, and
  automatically rebuilds them when the `Dockerfile` changes.
- **Seamless Container Interaction**: Starts, stops, and enters containers with
  a single command.
- **Engine Agnostic**: Works with both Docker and Podman, allowing you to choose
  your preferred container engine.
- **Workspace Integration**: Mounts your project directory into the container,
  so you can edit files on your host machine and see the changes reflected in
  the container.
- **Custom Command Execution**: Run any command inside your container
  environment without needing to SSH or use `docker exec`.

## Getting Started

### Prerequisites

Before you begin, ensure you have the following installed on your system:

- [Rust](https://www.rust-lang.org/tools/install) (for building from source)
- [Docker](https://docs.docker.com/get-docker/) or
  [Podman](https://podman.io/getting-started/installation)

### Installation

1.  **Clone the repository**:

    ```sh
    git clone https://github.com/your-username/containers.git
    cd containers
    ```

2.  **Build the project**:

    ```sh
    cargo build --release
    ```

3.  **Install the binary**: The executable will be located at
    `target/release/containers`. For convenient access, move it to a directory
    in your system's `PATH`.
    ```sh
    sudo mv target/release/containers /usr/local/bin/
    ```

## Usage

The basic syntax for `containers` is as follows:

```
containers [OPTIONS] [CONTAINER_NAME] [-- <COMMAND>...]
```

### Options

| Option                | Short | Description                                                                                                  |
| --------------------- | ----- | ------------------------------------------------------------------------------------------------------------ |
| `--dockerfile <PATH>` | `-f`  | Use a specific `Dockerfile`. By default, `containers` searches for a `Dockerfile` in the current directory.  |
| `--update`            | `-u`  | Force a rebuild of the image and recreation of the container.                                                |
| `CONTAINER_NAME`      |       | Set a custom name for the container. If not provided, the name is derived from the `Dockerfile`'s directory. |
| `-- <COMMAND>...`     |       | Run a custom command inside the container.                                                                   |

### Environment Variables

You can also configure `containers` using environment variables:

| Variable           | Description                                                                         |
| ------------------ | ----------------------------------------------------------------------------------- |
| `CONTAINER_NAME`   | Sets the default container name.                                                    |
| `DOCKERFILE`       | Sets the default `Dockerfile` path.                                                 |
| `CONTAINER_ENGINE` | Specifies the container engine to use (`docker` or `podman`). Defaults to `podman`. |

## Examples

### Basic Usage

- **Start or enter a container**: This command will automatically find the
  `Dockerfile`, build the image if it doesn't exist, and start or enter the
  container.

  ```sh
  containers
  ```

- **Use a custom container name**:
  ```sh
  containers my-dev-environment
  ```

### Advanced Usage

- **Specify a custom Dockerfile**:

  ```sh
  containers -f path/to/your/custom.dockerfile
  ```

- **Force an update**: If you've made changes to your `Dockerfile` and want to
  rebuild the image, use the `-u` flag.

  ```sh
  containers -u
  ```

- **Run a custom command**: You can execute any command within the container by
  appending it after `--`.

  ```sh
  containers -- ls -la
  ```

- **Use Docker as the container engine**:
  ```sh
  CONTAINER_ENGINE=docker containers
  ```

## How it Works

The `containers` utility is designed to be intuitive and intelligent. Here's a
breakdown of its workflow:

1.  **Configuration**: The tool starts by parsing command-line arguments and
    environment variables to establish its configuration.

2.  **Dockerfile Resolution**: It searches for a `Dockerfile` in the current
    directory. If one isn't found, it traverses up the directory tree until a
    `Dockerfile` is located.

3.  **Image Management**:
    - A lockfile (`.containers.lock`) is used to track the state of the
      `Dockerfile`.
    - If the `Dockerfile` has been modified since the last build, or if no image
      exists, `containers` will automatically build a new image.
    - This ensures that you are always working with an up-to-date environment
      without needing to manually rebuild images.

4.  **Container Lifecycle**:
    - **Creation**: If the container does not exist, a new one is created from
      the image. The directory containing the `Dockerfile` is mounted as a
      volume, allowing you to work on your files from your host machine.
    - **Starting**: If the container exists but is stopped, it is started.
    - **Entering**: If the container is already running, `containers` provides a
      shell into the container.

5.  **Command Execution**: If a command is provided, it is executed within the
    container's context. Otherwise, the default shell of the container is
    launched.

## Building from Source

If you want to contribute to the development of `containers` or simply build it
from source, follow these steps:

1.  **Clone the repository**:

    ```sh
    git clone https://github.com/your-username/containers.git
    cd containers
    ```

2.  **Build the project**:
    ```sh
    cargo build --release
    ```
    The resulting binary will be located at `target/release/containers`.

## Contributing

Contributions are welcome! If you have any ideas, suggestions, or bug reports,
please open an issue or submit a pull request.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file
for details.
