# Releasing

ozzyyaml ships on **crates.io** (Rust) and **PyPI** (Python). Version tags of
the form `v*` trigger both publish workflows.

## One-time: Trusted Publishing

Publishing uses OIDC — no long-lived API tokens in GitHub secrets.

### GitHub environment

In the repo: **Settings → Environments → New environment** → name it
`release`. (Optional but recommended: restrict to tags matching `v*`.)

### PyPI

1. Open [pypi.org/manage/project/ozzyyaml/settings/publishing](https://pypi.org/manage/project/ozzyyaml/settings/publishing)
2. Add a **Trusted publisher** (GitHub):
   - Owner: `peterkmurphy`
   - Repository: `ozzyyaml`
   - Workflow: `publish-pypi.yml`
   - Environment: `release`
3. You can delete the old `PYPI_API_TOKEN` GitHub secret afterward.

### crates.io

For **each** of `ozzyyaml-parser`, `ozzyyaml`, and `ozzyyaml-emit`:

1. Open the crate → **Settings** → **Trusted Publishing**
2. Add a GitHub Actions publisher:
   - Repository owner: `peterkmurphy`
   - Repository name: `ozzyyaml`
   - Workflow filename: `publish-crates.yml`
   - Environment: `release`
3. You can revoke the old crates.io API token used for the manual `0.1.0`
   upload if you no longer need it.

## Cutting a release

1. Bump `version` in the workspace `Cargo.toml` and in `python/pyproject.toml`
   to the same value (for example `0.2.0`).
2. Update path-dep version pins in `crates/ozzyyaml/Cargo.toml` and
   `crates/ozzyyaml-emit/Cargo.toml` to match.
3. Commit, push `main`, then tag:

```bash
git tag v0.2.0
git push origin v0.2.0
```

4. Watch:
   - [publish-crates.yml](../.github/workflows/publish-crates.yml)
   - [publish-pypi.yml](../.github/workflows/publish-pypi.yml)

PyPI builds Linux (x86_64 + aarch64), Windows (x64), and macOS (Apple Silicon)
wheels plus an sdist. crates.io publishes parser → ozzyyaml → emit, with
retries for index lag.
