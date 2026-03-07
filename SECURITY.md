# Security Policy

## Supported Versions

Security fixes are provided for the latest active release line.

| Version | Supported |
| --- | --- |
| 0.8.x | Yes |
| < 0.8 | No |

## Reporting a Vulnerability

Please avoid posting undisclosed vulnerabilities in public issues.

Preferred path:
- Open a private GitHub Security Advisory draft: `https://github.com/kungfulife/Hallinta/security/advisories/new`

Include:
- Affected version/commit
- Reproduction steps or proof-of-concept
- Impact assessment
- Suggested mitigation (if available)

## Security Model

Hallinta is a local desktop app with these key trust boundaries:

1. Local filesystem access
- Reads and writes Noita/Entangled Worlds data, `settings.json`, `presets.json`, and backup/snapshot ZIP files.

2. Modpacks network input
- Fetches Modpacks catalog JSON from a user-configured URL.
- Downloads preset JSON files from catalog `download_url` values (including transformed Google Drive links).

3. External handlers
- Opens local files/folders and external URLs/protocols via OS handlers (including `steam://subscribe/<id>`).

## Existing Protections in Code

- HTTP clients set explicit timeouts for catalog fetch and preset download requests.
- Catalog parsing requires `catalog_version` to be present.
- Preset imports validate expected export format (`hallinta_export == "presets"`) and non-empty payloads.
- SHA-256 checksum verification is implemented for preset imports.
  - Local file imports: checksum mismatch prompts user confirmation before import.
  - Downloaded Modpacks: checksum mismatch currently logs a warning.
- Save Monitor snapshot folder names are sanitized to avoid unsafe path characters.
- Backup deletion validates that the target file resolves under the backups directory.
- XML attribute escaping prevents malformed `mod_config.xml` output from untrusted mod names.

## Priority Hardening Recommendations

1. High: Block ZIP path traversal during restore
- `restore_backup()` currently joins archive entry names into target paths without canonical boundary checks.
- Add strict validation to reject `..`, absolute paths, and drive-qualified paths before writing.

2. High: Tighten Modpacks source trust
- Prefer/enforce HTTPS for catalog and download URLs.
- Consider an allowlist for trusted hostnames.
- Require successful checksum verification for downloaded Modpacks by default (with explicit override UI if needed).

3. Medium: Add authenticity guarantees
- Sign catalogs/modpack manifests (for example, detached Ed25519 signatures) and verify before import.

4. Medium: Add resource limits
- Limit downloaded payload size and reject oversized JSON responses.
- Consider bounded parsing/memory safeguards for malformed catalogs.

5. Medium: Validate external protocol inputs
- Validate workshop IDs (numeric/expected format) before constructing `steam://subscribe/...` URLs.

6. Low: Improve log privacy controls
- Provide an option to redact local paths/system identifiers for shared diagnostic logs.

## Secure Usage Guidance

- Treat Modpacks catalog sources as trusted inputs; use known hosts only.
- Keep routine backups before importing unknown Modpacks.
- Review checksum mismatch prompts/warnings carefully.
- Review logs before sharing externally (they may include local path/system metadata).

## Out of Scope

The following are outside Hallinta's direct control:
- A compromised local OS/user account
- Compromised Steam/Noita installations or third-party mod binaries
- Malicious software with local file write access to the same machine
