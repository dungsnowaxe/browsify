#!/bin/bash

fd . --type f packages/browsify-core packages/browsify-js apps/browsify apps/browsify-ui \
  | entr -r sh -c 'clear && ./scripts/build.sh && killall browsify && echo "$?"'
