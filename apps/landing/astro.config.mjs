import { defineConfig } from 'astro/config';
import { fileURLToPath } from 'node:url';

import cloudflare from '@astrojs/cloudflare';

const fromApp = (path) => fileURLToPath(new URL(path, import.meta.url));

export default defineConfig({
  output: 'static',
  session: false,

  vite: {
    server: {
      fs: {
        allow: [fromApp('.'), fromApp('../../assets/brand'), fromApp('../browsify-ui/public')],
      },
    },
  },

  adapter: cloudflare({ imageService: 'passthrough' }),
});