#!/bin/bash
set -e

# Update UID/GID if needed
if [ ! -z "$UID" ] && [ "$UID" != "$(id -u)" ]; then
    sudo usermod -u $UID code
fi

if [ ! -z "$GID" ] && [ "$GID" != "$(id -g)" ]; then
    sudo groupmod -g $GID code
    sudo usermod -g $GID code
fi

# Fix ownership of home directory
sudo chown -R code:code /home/code

exec "$@"
