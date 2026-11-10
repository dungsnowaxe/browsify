#!/bin/bash

fd . --type f packages/browsify-core packages/browsify-js apps/browsify apps/browsify-ui \
  | entr -r sh -c 'clear && DEBUG=true ./scripts/build.sh && ./apps/browsify/build/Browsify.app/Contents/MacOS/browsify && echo "$?"'
