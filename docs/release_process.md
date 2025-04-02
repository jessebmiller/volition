# Release Process

This project uses [release-please](https://github.com/googleapis/release-please) to automate the release process. Release-please creates and maintains release PRs that track all changes since the last release, updating the version according to [Semantic Versioning](https://semver.org/) principles and [Conventional Commits](https://www.conventionalcommits.org/).

## How It Works

1. When code is merged to the `main` branch, release-please checks if there are any releasable changes (commits with types like `feat`, `fix`, `docs`, or `deps`).

2. If releasable changes are found, release-please:
   - Updates the version in `Cargo.toml` files
   - Updates the `CHANGELOG.md` files
   - Creates or updates a Release PR

3. When the Release PR is approved and merged, release-please:
   - Creates a GitHub release
   - Creates git tags for the release
   - Triggers the publishing workflow to publish to crates.io

## Making Changes

When making changes, use the [Conventional Commits](https://www.conventionalcommits.org/) format for your commit messages:

- `feat: add new feature` - Bumps minor version (0.1.0 → 0.2.0)
- `fix: resolve bug` - Bumps patch version (0.1.0 → 0.1.1)
- `feat!: breaking change` or `feat: breaking change BREAKING CHANGE: description` - Bumps major version (0.1.0 → 1.0.0)
- `docs: update README` - No version bump, but included in changelog
- `chore: update dependencies` - No version bump or changelog entry

**Important**: The commit message format must be exactly as shown above. Release-please will fail to parse commits with formats like "trying release-please" or "updates release.yaml". The first word must be one of the conventional commit types (feat, fix, docs, etc.) followed by a colon.

Examples of correctly formatted commits:
```
feat: add new search feature
fix(cli): resolve panic when no arguments provided
docs: update installation instructions
chore: bump dependencies to latest versions
```

## Publishing to crates.io

Once a release is created on GitHub, the publishing workflow automatically:

1. Checks out the repository at the tagged commit
2. Builds and tests the code
3. Publishes to crates.io using the crates.io API token

## Manual Release (if needed)

If you need to trigger a release manually, you can:

1. Add a commit with `Release-As: x.y.z` in the commit body
2. For pre-releases, use `Release-As: x.y.z-pre.n`

## Setup

To set up release automation:

1. Add the GitHub Actions workflows (`.github/workflows/release-please.yml` and `.github/workflows/publish-crates.yml`)
2. Add the release-please configuration (`release-please-config.json` and `.release-please-manifest.json`)
3. Create a crates.io API token at https://crates.io/settings/tokens
4. Add the token as a GitHub secret named `CRATES_IO_TOKEN`
