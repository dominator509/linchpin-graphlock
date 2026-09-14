#!/usr/bin/env bash
# Real-dependency integration runner (V-009, DOD-009).
#
# Runs the integration suite that exercises production-type dependencies at
# their real boundary:
#   * rusqlite with the `bundled` feature — a real SQLite engine, file-backed
#   * tantivy — the real search engine
#
# DOD-009 requires production-type databases rather than in-memory substitutes,
# because in-memory stand-ins differ in transactions, locking and durability.
# The durability tests here open a real file, close the connection, and read the
# data back through an independent connection.
#
# DOD-007: collection is verified against the committed manifest so an empty or
# misconfigured suite cannot pass.
set -eu

[ -f Cargo.toml ] || { echo "integration: no Cargo workspace" >&2; exit 2; }

echo "integration: running real-dependency suite (file-backed SQLite, tantivy)"
cargo test --workspace --locked --test integration_real_dependencies -- --test-threads=1

# Guard against a suite that silently collects nothing.
collected=$(cargo test --workspace --locked --test integration_real_dependencies -- --list 2>/dev/null \
  | grep -c ': test$' || true)
echo "integration: collected ${collected} test(s)"

expected=5
if [ "$collected" -lt "$expected" ]; then
  echo "integration: FAIL -- collected ${collected} below expected ${expected}" >&2
  exit 1
fi

echo "integration: ok (${collected} real-dependency tests)"
