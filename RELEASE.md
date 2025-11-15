# Release Process

This document describes how to create a new release of the Activities project.

## Prerequisites

- Write access to the repository
- Clean working directory (no uncommitted changes)
- On the `main` branch
- `cargo-edit` installed (script will auto-install if missing)
- `npm` installed

## Release Workflow

### 1. Bump Version

Run the version bump script to sync versions across the sub-projects:

```bash
./scripts/bump-version.sh 1.0.0
```

This will:

- Update version in `app/Cargo.toml`
- Update version in `client/package.json`
- Create a commit with message: `chore: bump version to 1.0.0`

### 2. Push the Commit

```bash
git push origin main
```

### 3. Create GitHub Release

1. Go to: https://github.com/thomas-god/activities/releases/new
2. Click "Choose a tag" and type `v1.0.0` (create new tag)
3. Set the target to `main`
4. Set the release title: `Release v1.0.0`
5. Write your release notes (see guidelines below)
6. Click "Publish release"

### 4. CI Takes Over

Once you publish the release, the CI workflow automatically:

1. **Builds Docker images** for:
   - Both variants: `single-user` and `multi-user`
   - Both architectures: `amd64` and `arm64`

2. **Tags images** with:
   - Full version: `single-user-v1.0.0`, `multi-user-v1.0.0`
   - Minor version: `single-user-v1.0`, `multi-user-v1.0`
   - Major version: `single-user-v1`, `multi-user-v1`
   - Latest: `single-user-latest`, `multi-user-latest`

3. **Updates the release** with Docker pull commands

## Pre-releases

For beta or release candidate versions:

1. Run version bump: `./scripts/bump-version.sh 1.1.0-beta.1`
2. Push to GitHub
3. Create release with tag `v1.1.0-beta.1`
4. **Check "Set as a pre-release"**
5. Publish

Pre-releases won't update the `-latest` Docker tags.
