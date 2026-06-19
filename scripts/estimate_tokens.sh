#!/usr/bin/env sh
set -eu

if [ "$#" -eq 0 ]; then
  set -- src src-tauri
fi

printf '%-64s %12s\n' DIR APPROX_TOKENS

for dir in "$@"; do
  [ -d "$dir" ] || continue
  bytes=$(find "$dir" \
    \( -path '*/node_modules' -o -path '*/node_modules/*' -o \
       -path '*/.git' -o -path '*/.git/*' -o \
       -path '*/.claude' -o -path '*/.claude/*' -o \
       -path '*/.planning' -o -path '*/.planning/*' -o \
       -path '*/target' -o -path '*/target/*' -o \
       -path '*/dist' -o -path '*/dist/*' -o \
       -path '*/build' -o -path '*/build/*' -o \
       -path '*/.svelte-kit' -o -path '*/.svelte-kit/*' \) -prune -o \
    -type f \
    \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' -o -name '*.js' -o -name '*.jsx' -o -name '*.svelte' -o -name '*.md' -o -name '*.json' \) \
    -exec cat {} + 2>/dev/null | wc -c | tr -d ' ')
  tokens=$(( (bytes + 3) / 4 ))
  printf '%-64s %12s\n' "$dir" "$tokens"
done
