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

# Install Claude Code CLI globally
RUN npm install -g @anthropic-ai/claude-code

# Set working directory
WORKDIR /workspace

CMD ["/bin/bash"]
