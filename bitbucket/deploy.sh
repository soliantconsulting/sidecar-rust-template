#!/usr/bin/env bash

set -e

ENV=$1

if [ "$ENV" == "staging" ]; then
  export SIDECAR_VERSION=main
elif [ "$ENV" == "production" ]; then
  pnpm semantic-release --dry-run
  source /tmp/sidecar.env
  pnpm semantic-release-cargo prepare "$SIDECAR_VERSION"
  pnpm semantic-release
else
  echo "Invalid environment: $ENV"
  exit 1
fi

pnpm sidecar-deploy
