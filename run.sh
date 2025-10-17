#!/bin/bash

# Parse arguments and environment variables
CONTAINER_NAME="${CONTAINER_NAME:-dev-env}"
if [ "$1" != "" ]; then
    CONTAINER_NAME="$1"
fi
IMAGE_NAME="dev-env:latest"
DOCKERFILE="${DOCKERFILE:-Dockerfile}"
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

# Build image if Dockerfile exists and image doesn't exist
if [ -f "$DOCKERFILE" ]; then
    if ! $CONTAINER_ENGINE images --format "table {{.Repository}}:{{.Tag}}" | grep -q "${IMAGE_NAME}$\|localhost/${IMAGE_NAME}$"; then
        echo "Building image: ${IMAGE_NAME}"
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
