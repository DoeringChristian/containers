#!/bin/bash

# Parse arguments and environment variables
UPDATE_IMAGE=false

# Function to find Dockerfile recursively upwards
find_dockerfile() {
    local dir="$(pwd)"
    local home_dir="$HOME"
    while [ "$dir" != "/" ] && [ "$dir" != "$home_dir" ]; do
        if [ -f "$dir/Dockerfile" ]; then
            echo "$dir/Dockerfile"
            return 0
        fi
        dir="$(dirname "$dir")"
    done
    # Check home directory
    if [ -f "$home_dir/Dockerfile" ]; then
        echo "$home_dir/Dockerfile"
        return 0
    fi
    # Fallback to script directory
    local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    echo "$script_dir/Dockerfile"
}

DOCKERFILE="${DOCKERFILE:-$(find_dockerfile)}"

# Set default container name based on Dockerfile directory (full path, replace / with -)
DEFAULT_CONTAINER_NAME="$(dirname "$DOCKERFILE" | sed 's|^/||' | sed 's|/|-|g')"
CONTAINER_NAME="${CONTAINER_NAME:-$DEFAULT_CONTAINER_NAME}"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
    -f | --dockerfile)
        DOCKERFILE="$2"
        shift 2
        ;;
    -u | --update)
        UPDATE_IMAGE=true
        shift
        ;;
    *)
        CONTAINER_NAME="$1"
        shift
        ;;
    esac
done

IMAGE_NAME="dev-env:latest"
CONTAINER_ENGINE="${CONTAINER_ENGINE:-podman}"

# Detect NVIDIA GPU support
NVIDIA_ARGS=""
if command -v nvidia-smi &>/dev/null && nvidia-smi &>/dev/null; then
    if [ "$CONTAINER_ENGINE" = "docker" ]; then
        NVIDIA_ARGS="--gpus all"
    elif [ "$CONTAINER_ENGINE" = "podman" ]; then
        NVIDIA_ARGS="--device nvidia.com/gpu=all --security-opt=label=disable"
    fi
fi

# Build image if Dockerfile exists and (image doesn't exist or update requested)
if [ -f "$DOCKERFILE" ]; then
    if [ "$UPDATE_IMAGE" = true ] || ! $CONTAINER_ENGINE images --format "table {{.Repository}}:{{.Tag}}" | grep -q "${IMAGE_NAME}$\|localhost/${IMAGE_NAME}$"; then
        if [ "$UPDATE_IMAGE" = true ]; then
            echo "Updating image: ${IMAGE_NAME}"
            # Remove existing container if it exists
            if $CONTAINER_ENGINE ps -a --format "table {{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
                echo "Removing existing container: ${CONTAINER_NAME}"
                $CONTAINER_ENGINE rm -f ${CONTAINER_NAME}
            fi
        else
            echo "Building image: ${IMAGE_NAME}"
        fi
        $CONTAINER_ENGINE build -t ${IMAGE_NAME} -f ${DOCKERFILE} .
    fi
fi

# Check if container exists
if $CONTAINER_ENGINE ps -a --format "table {{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
    # Container exists, check if it's running
    if $CONTAINER_ENGINE ps --format "table {{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
        echo "Entering running container: ${CONTAINER_NAME}"
        $CONTAINER_ENGINE exec -it ${CONTAINER_NAME} /bin/bash
    else
        echo "Starting existing container: ${CONTAINER_NAME}"
        $CONTAINER_ENGINE start ${CONTAINER_NAME}
        $CONTAINER_ENGINE exec -it ${CONTAINER_NAME} /bin/bash
    fi
else
    echo "Creating new container: ${CONTAINER_NAME}"
    $CONTAINER_ENGINE run -it --name ${CONTAINER_NAME} \
        -v "$(pwd)":"$(pwd)" \
        -w "$(pwd)" \
        ${NVIDIA_ARGS} \
        ${IMAGE_NAME} /bin/bash
fi
