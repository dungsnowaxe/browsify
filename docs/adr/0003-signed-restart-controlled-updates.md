# Signed Updates with Explicit Restart Approval

Browsify automatically checks for and downloads stable releases, but installs only after explicit Restart to Update approval; disabling Automatic Updates cancels background work and discards automatically staged packages while preserving manual checks. Update archives require an Ed25519 signature verified against a public key embedded in the installed executable, rather than trusting a checksum delivered alongside the archive, because our releases are not Apple-signed or notarized.

The initial scope is writable installed app bundles without privilege escalation: stage and validate before replacing, retain the previous bundle for rollback, and preserve pending link requests across restart. Existing check-only installations require a one-time manual upgrade, and release signing must be provisioned separately; missing signing configuration must never enable unsigned automatic installation.
