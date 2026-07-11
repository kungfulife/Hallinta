# Windows Auto-Update

Official Windows release builds check `kungfulife/Hallinta` GitHub Releases at startup. Debug builds and ordinary local release builds do not update. The release workflow sets `HALLINTA_DIST_BUILD=true` when compiling the downloadable executable.

## Release contract

- Stable semantic-version tag, such as `v0.8.1`.
- Exact asset name: `Hallinta-x86_64-pc-windows-msvc.exe`.
- GitHub-provided `sha256:` asset digest must be present.
- The asset URL must belong to this repository's GitHub Releases download path.
- Drafts, prereleases, equal versions, and downgrades are ignored.

The release workflow builds on Windows, creates a draft release, uploads the portable executable, compares GitHub's asset digest with the local SHA-256, and publishes only after they match. A manual workflow run exercises the same draft/upload/digest path and deletes its temporary validation release and tag.

## Client safety sequence

1. Download to a unique sibling file while showing progress. Cancellation is allowed only during this phase.
2. Freeze ordinary Save Monitor snapshots once the user accepts, then verify size and SHA-256 while downloading.
3. Wait for any current snapshot or backup operation. If monitoring is active, require one final snapshot; failure leaves Hallinta open.
4. Copy the running executable as a helper, launch it outside a compatible Windows job, and require a live PID/creation-time acknowledgement.
5. Close the UI. The helper waits for the original PID and the single-instance lock, re-hashes the staged file under that lock, then calls `ReplaceFileW` with a rollback path.
6. Launch the new executable and retain the rollback copy until the new UI renders its first frame and signals readiness.
7. If launch/readiness fails, restore the previous executable and restart it with a visible error. An active monitor session is resumed after either success or failure.

The interaction-blocking update UI is independent of normal application modals. Window close and application shortcuts are suppressed from accepted download through helper takeover.
