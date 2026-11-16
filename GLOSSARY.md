# Browsify

Browsify is a URL router and browser profile launcher that intercepts outbound links, applies URL rewrites, and opens the resolved destination in the right browser, profile, container, or web app.

## Language

### Configuration & Routing

**Opener**:
The external application that initiated a URL open request, characterized by its application name, bundle identifier, path, and optional focused window title.
_Avoid_: Caller, sender, source app

**Primary Config**:
The user-authored configuration file that defines the highest-priority rewrites, handlers, options, and default browser.
_Avoid_: Script, main config

**UI Rules File**:
The visual-editor rules configuration evaluated as fallback handlers and fallback defaults after the Primary Config.
_Avoid_: Cached rules, secondary config

**Rewrite Rule**:
A rule that matches an incoming URL and mutates or replaces it before handler evaluation.
_Avoid_: Redirect, transform rule, modifier

**Handler Rule**:
A rule that matches a post-rewrite URL and Opener to select a Browser Target.
_Avoid_: Route, matcher rule

### Browser & Profile Resolution

**Browser Target**:
The logical destination selected by a Handler Rule or default browser—specifying the browser identity, optional Profile Selector, Container, web app mode, custom arguments, and foreground/background preference—or suppression of the launch.
_Avoid_: Browser config, browser spec

**Profile Selector**:
The user-specified name, email address, directory key, path, or container prefix that identifies a desired browser profile prior to resolution.
_Avoid_: Profile flag, profile ID

**Launch Plan**:
The concrete execution plan produced by resolving a Browser Target and its Profile Selector into final launch arguments, target URL, instance-spawning behavior, and window activation policy.
_Avoid_: Launch config, open command

### Browsify Updates

**Automatic Updates**:
Browsify's opt-out background discovery and download of new releases, with installation requiring explicit restart approval.
_Avoid_: Automatic installation, update checks

**Skipped Release**:
A Browsify release the user has chosen to exclude from automatic update prompts and downloads, but which remains discoverable through a manual check.
_Avoid_: Dismissed update, deferred update

**Deferred Update**:
An available Browsify update whose next automatic prompt the user has postponed for 24 hours without skipping the release.
_Avoid_: Skipped release
