# Releasing AniTrax

M8's pipeline: pushing a `vX.Y.Z` tag triggers
`.github/workflows/release.yml`, which builds signed Windows/macOS/Linux
bundles via `tauri-apps/tauri-action` and publishes them as a **draft**
GitHub Release with the `latest.json` manifest the in-app updater checks
against (`src-tauri/tauri.conf.json`'s `plugins.updater.endpoints`). Nothing
is live — no installed copy sees the update — until that draft is manually
published.

## Cutting a release

1. Bump `version` together in all three places (they must match):
   - `src-tauri/tauri.conf.json`
   - `package.json`
   - `src-tauri/Cargo.toml` (`[package].version`)
2. Commit the bump.
3. Tag and push:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```
4. Watch the **Release** workflow run (Actions tab) — it builds all three
   platforms in parallel and uploads bundles + `latest.json` to a new draft
   release named after the tag.
5. Review the draft release on GitHub (add real release notes if you want
   better text than the generic body the workflow writes), then **Publish**
   it. Publishing is what makes `latest.json` resolve at the `endpoints` URL
   and starts offering the update to installed copies.

## Signing keys

Update artifacts are signed with an Ed25519 keypair (`tauri signer
generate`). The public key lives in `tauri.conf.json`; the private key +
its password are GitHub Actions secrets on the repo
(`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) — never
committed. Losing the private key means a new keypair (and a new pubkey in
`tauri.conf.json`) is needed, and existing installs can't verify updates
signed with the old key until they're manually reinstalled with a build
carrying the new pubkey.

## First real release — acceptance still owed

The full updater loop (an installed build detects the new version at
launch, shows the prompt with release notes, downloads, installs, and
relaunches on the new version; an unsigned/tampered bundle is rejected)
can't be exercised until a real tag has gone through this pipeline and been
published — flagged as acceptance-owed in `docs/milestones.md`'s M8 section,
same pattern used for M6/M7's own deferred live-test passes.
