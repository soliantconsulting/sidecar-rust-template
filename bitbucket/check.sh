#!/usr/bin/env bash

return_code=0

commands=(
    "pnpm biome ci ."
    "pnpm tsc --noEmit"
    "cargo fmt --check"
    "cargo check"
    "cargo clippy"
)

for cmd in "${commands[@]}"; do
    echo "Running $cmd"
    $cmd

    if [ $? -ne 0 ]; then
        return_code=1
    fi
done

exit $return_code
