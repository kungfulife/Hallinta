# Windows Auto-Update

Official Windows distribution builds check public `kungfulife/Hallinta` GitHub
Releases at startup. Debug builds and ordinary local release builds do not
check for or install updates. The release workflow marks official builds with
`HALLINTA_DIST_BUILD=true`.

## Release contract

- Releases use stable semantic-version tags such as `v0.8.3`.
- The manual portable download remains
  `Hallinta-x86_64-pc-windows-msvc.exe`.
- The updater requires the exact signed archive
  `Hallinta-x86_64-pc-windows-msvc.zip`, containing only `Hallinta.exe`.
- Hallinta offers only a stable version newer than the running version.
- `Cargo.toml` is the sole application-version source. Windows File version
  and Product version are built from the same value.

Release CI builds both artifacts, verifies the executable's embedded version
and identity, signs the updater ZIP with zipsign, verifies that signature and
archive contents, uploads a draft, verifies both GitHub asset digests, and only
then publishes the release.

## Client sequence and ownership

1. Hallinta selects a newer stable release with the exact updater ZIP and asks
   for user consent. **Dismiss** stores that version so automatic startup checks
   stay quiet; a newer version prompts again, and Settings → Check for Updates
   always re-offers the current candidate.
2. Hallinta blocks ordinary actions, waits for backup/restore work, and takes
   one final snapshot when Save Monitor is active. Snapshot failure leaves the
   app open and does not start installation.
3. `self_update` downloads the chosen release, verifies its embedded zipsign
   signature, extracts `Hallinta.exe`, replaces the running executable through
   `self-replace`, and cleans its temporary files.
4. Hallinta closes, releases the `hallinta_noita` single-instance lock, and
   launches the installed executable. An active monitor session resumes from
   explicit preset and session arguments.
5. On first launch of the new version, Hallinta writes a version-upgrade backup
   into the shared `backups/` folder (same surface as Manage Backups). Legacy
   archives under `upgrade_backups/` remain listable and restorable.

Hallinta contains no download loop, staging convention, replacement API,
helper protocol, readiness handshake, or automatic executable rollback engine.
An accepted install is intentionally non-cancellable and displays indeterminate
progress while the external updater owns the transaction.

## Signing trust

zipsign provides free Ed25519 update authenticity against a public key embedded
in Hallinta. It is not Microsoft Authenticode publisher signing, so Windows may
still show `Unknown publisher` or SmartScreen warnings.

The private signing key exists only in protected operator custody and the
`HALLINTA_ZIPSIGN_PRIVATE_KEY_B64` GitHub Actions secret. Key rotation requires
first shipping a Hallinta version that trusts both old and new public keys,
then signing later releases only with the new key.
