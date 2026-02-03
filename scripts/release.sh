#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
ARCHIVE_NAME="seloria-release.tar.gz"

mkdir -p "${DIST_DIR}"

echo "Building Seloria release binary..."
(cd "${ROOT_DIR}" && cargo build --release)

echo "Copying artifacts..."
cp "${ROOT_DIR}/target/release/seloria" "${DIST_DIR}/seloria"
cp "${ROOT_DIR}/docs/CONFIG_TEMPLATE.json" "${DIST_DIR}/config.json"
cp "${ROOT_DIR}/docs/OPENCLAW_AGENTS.md" "${DIST_DIR}/OPENCLAW_AGENTS.md"
cp "${ROOT_DIR}/docs/COMMITTEE_EC2_LOCAL.md" "${DIST_DIR}/COMMITTEE_EC2_LOCAL.md"

echo "Packaging ${ARCHIVE_NAME}..."
tar -czf "${DIST_DIR}/${ARCHIVE_NAME}" -C "${DIST_DIR}" \
  seloria config.json OPENCLAW_AGENTS.md COMMITTEE_EC2_LOCAL.md

echo "Release package created at ${DIST_DIR}/${ARCHIVE_NAME}"
