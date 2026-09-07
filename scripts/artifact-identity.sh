#!/usr/bin/env bash
set -eu
f="${1:?artifact path}"; sha256sum "$f"
