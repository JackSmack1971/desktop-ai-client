#!/usr/bin/env sh
set -eu

root=${1:-.}

has_root_agents=0
has_root_claude=0
has_child_agents=0

[ -f "$root/AGENTS.md" ] && has_root_agents=1
[ -f "$root/CLAUDE.md" ] && has_root_claude=1

if [ "$has_root_agents" -eq 1 ] || [ "$has_root_claude" -eq 1 ]; then
  if find "$root" -mindepth 2 -name AGENTS.md -print -quit | grep -q .; then
    has_child_agents=1
  fi
fi

if [ "$has_root_agents" -eq 0 ] && [ "$has_root_claude" -eq 0 ]; then
  printf '%s\n' none
elif [ "$has_root_agents" -eq 1 ] && [ "$has_root_claude" -eq 0 ] && [ "$has_child_agents" -eq 1 ]; then
  printf '%s\n' complete
else
  printf '%s\n' partial
fi
