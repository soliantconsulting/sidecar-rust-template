#!/usr/bin/env bash

if [ -f target-cache/target.tar ]; then
    echo "Restoring target directory from target.tar…"
    tar -xf target-cache/target.tar
else
    echo "No previous target found, starting fresh."
fi
