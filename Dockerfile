FROM ubuntu:25.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    # Build tools
    clang-18 \
    cmake \
    ninja-build \
    git \
    # Python
    python3 \
    python3-pip \
    python3-dev \
    # Image libraries
    libpng-dev \
    libjpeg-dev \
    # Testing tools
    python3-pytest \
    python3-pytest-xdist \
    python3-numpy \
    # Additional utilities
    curl \
    wget \
    vim \
    # Node.js for Claude Code
    nodejs \
    npm \
    && rm -rf /var/lib/apt/lists/*

# Set Clang as default compiler
ENV CC=clang-18
ENV CXX=clang++-18

# Create symlinks for python
RUN ln -sf /usr/bin/python3 /usr/bin/python

# Install Rust globally (will be accessible to all users)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile default
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Claude Code CLI globally
RUN npm install -g @anthropic-ai/claude-code

# Create a non-root user with matching host UID/GID
ARG UID=1000
ARG GID=1000
RUN if getent passwd $UID > /dev/null 2>&1; then \
        # User with this UID already exists, use it
        USER_NAME=$(getent passwd $UID | cut -d: -f1); \
    else \
        # Create new user
        if getent group $GID > /dev/null 2>&1; then \
            GROUP_NAME=$(getent group $GID | cut -d: -f1); \
        else \
            groupadd -g $GID claude-user && GROUP_NAME=claude-user; \
        fi && \
        useradd -m -s /bin/bash -u $UID -g $GROUP_NAME claude-user && \
        USER_NAME=claude-user; \
    fi && \
    echo "$USER_NAME ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

