#!/usr/bin/env bash

# This requires the following to have been run at some point before running this script:
# docker buildx create --name builder --driver docker-container --use

readonly _tag
_tag=$(git describe --tags --abbrev=0 | sed 's/v//g')
docker buildx build \
  -t "oddlid/rlunch:$_tag" \
  --build-arg VERSION="$_tag" \
  --build-arg BUILD_DATE="$(date --rfc-3339=ns)" \
  --build-arg VCS_REF="$(git rev-parse --short HEAD)" \
  --platform=linux/amd64,linux/arm64 \
  --push .
