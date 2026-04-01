#!/usr/bin/env bash

# Rename .st -> .sx recursively
find . -type f -name "*.st" -print0 | while IFS= read -r -d '' file; do
  new="${file%.st}.sx"
  mv "$file" "$new"
done