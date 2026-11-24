#!/bin/bash

fd . --type f packages/core packages/browsify-js apps/cli apps/ui \
  | entr -r sh -c 'clear && ./scripts/build.sh && killall browsify && echo "$?"'
