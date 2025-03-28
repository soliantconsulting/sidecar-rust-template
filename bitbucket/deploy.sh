#!/usr/bin/env bash

set -e

ENV=$1

if [ "$ENV" == "staging" ]; then
  export SIDECAR_VERSION=main
elif [ "$ENV" == "production" ]; then
  if pnpm semantic-release --dry-run | grep -q "No release published"; then
    echo "No new release. Aborting."
    exit 1
  fi

  source /tmp/sidecar.env
  semantic-release-cargo prepare "$SIDECAR_VERSION"
  pnpm semantic-release
else
  echo "Invalid environment: $ENV"
  exit 1
fi

pnpm sidecar-deploy
