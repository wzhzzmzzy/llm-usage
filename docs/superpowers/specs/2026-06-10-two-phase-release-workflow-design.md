# Two-Phase Tauri Release Workflow

**Date:** 2026-06-10  
**Status:** Approved

## Problem

The existing `release.yml` runs `tauri-apps/tauri-action` in a 3-platform matrix where each job tries to create the same GitHub Release independently. This causes race conditions: jobs collide on the same tag, some uploads succeed and others fail.

## Design

Split into two sequential jobs:

### Job 1: `create-release`

- Trigger: push to `main`
- Reads `package.version` from `src-tauri/tauri.conf.json` using `jq`
- Creates a GitHub Release via `softprops/action-gh-release` with:
  - Tag: `v<version>` (e.g. `v0.1.0`)
  - `draft: false`, `prerelease: false`
  - Empty body initially; each platform upload appends its assets
- Outputs `release_id` for downstream jobs

### Job 2: `build-tauri` (3-platform matrix)

- `needs: create-release`
- Matrix: Linux x86_64, macOS aarch64, Windows x86_64
- Each job builds with `tauri-apps/tauri-action@v0`, passing:
  - `releaseId: ${{ needs.create-release.outputs.release_id }}` — targets the existing release instead of creating a new one
  - No `tagName` / `releaseName` — avoids collision

### Expected artifacts per platform

| Platform | Files |
|----------|-------|
| macOS aarch64 | `.dmg`, `.app.tar.gz` |
| Linux x86_64 | `.deb`, `.AppImage` |
| Windows x86_64 | `.msi`, `.exe` (NSIS) |

## Constraints

- Version source of truth: `src-tauri/tauri.conf.json` → `package.version`
- `permissions: contents: write` already set at workflow level
- No code signing — unsigned builds, macOS Gatekeeper warning expected
