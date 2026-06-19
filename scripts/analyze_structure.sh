#!/usr/bin/env sh
set -eu

root=${1:-.}

printf '%-64s %7s %6s\n' DIR FILES AGENTS

find "$root" \
  \( -path '*/node_modules' -o -path '*/node_modules/*' -o \
     -path '*/.git' -o -path '*/.git/*' -o \
     -path '*/.claude' -o -path '*/.claude/*' -o \
     -path '*/.planning' -o -path '*/.planning/*' -o \
     -path '*/target' -o -path '*/target/*' -o \
     -path '*/dist' -o -path '*/dist/*' -o \
     -path '*/build' -o -path '*/build/*' -o \
     -path '*/.svelte-kit' -o -path '*/.svelte-kit/*' \) -prune -o \
  -type f \
  \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' -o -name '*.js' -o -name '*.jsx' -o -name '*.svelte' \) \
  | sed 's#[/\\][^/\\]*$##' \
  | sort \
  | uniq -c \
  | while read -r count dir; do
    [ -n "$dir" ] || continue
    if [ -f "$dir/AGENTS.md" ]; then
      agents=yes
    else
      agents=no
    fi
    printf '%-64s %7s %6s\n' "$dir" "$count" "$agents"
  done
