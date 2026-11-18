# Move to macOS 13 through a signed updater bridge

Open at Login uses the system login item API on macOS 13 and later instead of maintaining a legacy login mechanism. Publish a macOS 12-compatible, signed updater bridge before switching to the macOS 13-only archive name: already-shipped clients recognize only the legacy name and must never install an incompatible bundle. Keep the bridge available and check the required macOS version again before staging or replacing the installed app.
