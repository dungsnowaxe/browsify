#!/bin/bash

fd . --type f packages/core packages/browsify-js apps/cli apps/ui \
  | entr -r sh -c 'clear && DEBUG=true ./scripts/build.sh && ./apps/cli/build/Browsify.app/Contents/MacOS/browsify && echo "$?"'
