#!/bin/bash
set -e

# Use UID and GID from environment

groupadd -g ${GID} code 2>/dev/null || true
useradd -m -s /bin/bash -u ${UID} -g ${GID} code 2>/dev/null || true

# Execute the command passed to the container
exec "$@"
