# Update signing and rollout

Browsify verifies a raw 64-byte detached Ed25519 signature over the exact bytes of
`Browsify-macos13-arm64.tar.gz` (or `Browsify-arm64.tar.gz` for the macOS 12 bridge). The trusted key is a base64-encoded raw 32-byte Ed25519
public key, embedded at compile time using `BROWSIFY_UPDATE_PUBLIC_KEY`. A checksum
alone does not authorize installation. The signed bundle must match the advertised
version and Browsify bundle identifier; older and prerelease versions are excluded.

## Is an Apple Developer account required?

No Apple account or paid membership is required for the Ed25519 package signatures
used by this updater. These signatures establish that a package was authorized by
Browsify's release key; they do not establish Apple-verified developer identity or
remove Gatekeeper warnings.

For the normal trusted macOS distribution experience, add Developer ID Application
signing and Apple notarization separately. Developer ID certificates require Apple
Developer Program membership (a free Apple developer account is not sufficient).
See [Apple's Developer ID guide](https://developer.apple.com/developer-id/).

## Provisioning (maintainer action)

Use a securely generated Ed25519 key pair, keeping the private key out of the
repository, agent sessions, logs, and ordinary build jobs. An OpenSSL 3-compatible
PKCS#8 PEM private key is required by the publish job.

Configure these GitHub repository settings manually:

- Actions variable `BROWSIFY_UPDATE_PUBLIC_KEY`: base64 of the raw 32-byte public
  key, without line breaks (not the PEM or full DER SubjectPublicKeyInfo).
- Actions secret `BROWSIFY_UPDATE_PRIVATE_KEY_BASE64`: base64 of the complete
  PKCS#8 PEM private-key file.

The release build requires the public key. The publish job checks that its signer
matches that embedded key, signs the archive with `openssl pkeyutl -sign -rawin`,
verifies the signature, then uploads the archive, signature, and checksums together.
Private-key files use restrictive permissions and are removed by an EXIT trap.
Release runs fail rather than silently publishing unsigned updates. Local builds
without a public key remain usable but must be updated manually.

Signing credentials have not been provisioned by this change. Do not publish an
updater-enabled release until repository settings have been configured and a
staged release has passed the acceptance checks below.

### One-time local setup on macOS

Run this **yourself** in a normal terminal, not an agent session. It requires an
authenticated `gh` CLI with permission to manage repository Actions settings, plus
Homebrew OpenSSL 3 (`brew install openssl@3` if missing). It creates the private
key outside the repository and streams it directly into GitHub's encrypted secret
storage without printing its contents or putting the value into shell history.

```sh
(
  set -euo pipefail
  umask 077
  OPENSSL="$(brew --prefix openssl@3)/bin/openssl"
  KEY_DIR="$HOME/.local/share/browsify-signing"
  mkdir -p "$KEY_DIR"
  chmod 700 "$KEY_DIR"
  if [ -e "$KEY_DIR/update-private.pem" ]; then
    echo 'Signing key already exists; reuse it rather than overwriting it.' >&2
    exit 1
  fi

  "$OPENSSL" genpkey -algorithm Ed25519 -out "$KEY_DIR/update-private.pem"
  "$OPENSSL" pkey -in "$KEY_DIR/update-private.pem" -pubout -outform DER \
    | tail -c 32 | "$OPENSSL" base64 -A \
    | gh variable set BROWSIFY_UPDATE_PUBLIC_KEY --repo dungsnowaxe/browsify
  "$OPENSSL" base64 -A -in "$KEY_DIR/update-private.pem" \
    | gh secret set BROWSIFY_UPDATE_PRIVATE_KEY_BASE64 --app actions --repo dungsnowaxe/browsify
)
```

Keep an encrypted backup of `update-private.pem`; never commit, paste, or attach it
to a PR. Base64 is encoding, not encryption. If upload fails after generation,
reuse the existing key for the upload steps rather than generating another pair.
Do not rerun generation for each release: installed clients trust the same public
key across releases. Repository settings should show the variable and secret names;
there is no need to retrieve or share the stored secret value.

## Bootstrap and rotation

Existing check-only versions cannot install themselves: manually install the first
updater-enabled release. Never swap the signing key without an explicit migration:
installed clients trust their embedded key, not a key fetched alongside an update.
For rotation, deliver a transition release signed with the currently trusted key
that embeds the next public key, then change the release signer for later releases.
Clients that skip the transition will require a manual upgrade; supporting multiple
trusted keys would need a separate rotation design. A compromised signing key
requires out-of-band recovery, not an unsigned fallback.

## State and safety

The existing `checkForUpdates` / `check_for_updates` configuration keys retain their
precedence and persisted values; the UI labels the combined behavior Automatic
updates. Daily-check, skipped-version, and reminder preferences are stored under
`~/Library/Application Support/Browsify/`. Verified staged packages and their ready
record are private files under `~/Library/Caches/Browsify/updates/`; an automatic
package is discarded if the preference is disabled. Manually requested packages
remain available for explicit installation.

The installer runs as the user, with no administrator prompts. It verifies again,
extracts into a private sibling of the installed app, and rejects archive links,
foreign roots, special files, traversal, and oversized packages. It does not touch
browser profiles, routing configuration, or LaunchServices defaults. Pending URLs
received during restart preparation are written before acknowledgement and replayed
by the relaunched app. The OS-delivery gap during process handoff still requires
live macOS validation; unit tests do not prove AppleEvent delivery.

The old bundle is retained until the new process acknowledges AppKit startup. Failed
replacement/startup attempts restore or relaunch the old app where possible. If
filesystem recovery itself fails, the private `.browsify-install-*` sibling is
retained rather than deleting the only recovery copy. This is startup rollback,
not a guarantee against crashes after startup or power loss during replacement.

## Release acceptance (not yet performed)

Follow `.agents/skills/browsify-macos-testing/SKILL.md` before live tests. Use owned
test bundles and explicit configuration paths; never change GUI HOME or default
handlers casually.

- Signed stable update on Apple Silicon; corrupt archive/signature and
  mismatched signed version rejected without modifying the installed app.
- Native dialog actions, release notes, 24-hour deferral, skipping, and manual
  override; normal quit must not install.
- Disable during check, download, and ready prompt; no further automatic network
  activity and no stale prompt or staged automatic package.
- Manual check/download/install with automatic updates disabled, including offline
  failures, retry, protected locations, disk-image/translocated apps, and missing key.
- Restart with an active routing operation and incoming AppleEvents, startup
  failure/rollback, and preservation of configuration and both effective HTTP/HTTPS
  default handlers. Verify actual delivery; a successful helper exit is not proof.
- Quit/relaunch with a deferred package; opt-out after relaunch; verify private
  staging cleanup and retained recovery files on failed restoration.
