# Browsify landing site

Static Astro app. Use Node 24.22.1 and pnpm 11.20.0. Run these commands from the repository root:

```sh
pnpm -C apps/landing install --frozen-lockfile
pnpm -C apps/landing dev
pnpm -C apps/landing build
pnpm -C apps/landing preview
```

This app has its own pnpm lockfile; it does not change the desktop build or its lockfile. Build from a checkout of the repository so Astro can bundle the existing brand, font, and browser icon assets.

## Deploy to Cloudflare Workers

After a successful build, check the deployment locally:

```sh
pnpm -C apps/landing exec wrangler deploy --dry-run
```

When ready to publish, authenticate with `pnpm -C apps/landing exec wrangler login` and rerun the deploy command without `--dry-run`. Astro generates the deployment config for the `browsify-landing` Worker and serves the prerendered files from `dist/client/`. No KV namespace or Cloudflare Images binding is needed.

## Animation and assets

The Signal board uses CSS for its animation and a small browser script for pause/play, visibility, and reduced-motion preferences. The routing examples remain readable without JavaScript. The browser icons and their attribution are sourced from `apps/ui/public/icons/`; font licenses are included with the build.
