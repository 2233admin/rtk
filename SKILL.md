---
name: rtk
description: CLI proxy that reduces LLM token consumption by 60-90%. Prefix any dev command with 'rtk' to get filtered, compact output. Use for all Bash commands to save context window.
---

# rtk — Token-Optimized CLI Proxy

Single Rust binary. Zero dependencies. Prefix any command with `rtk` to get compact, LLM-friendly output.

## Install

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/2233admin/rtk/main/install.sh | bash

# Windows (scoop)
scoop install rtk

# ClawHub (skill only, binary installed separately)
clawhub install rtk
```

## Core Usage

Just prefix your commands with `rtk`:

```bash
rtk git status          # compact git output
rtk git log --oneline   # stripped decorations
rtk gh pr list          # token-optimized GitHub CLI
rtk docker ps           # compact container list
rtk npm test            # only show failures
rtk cargo build         # compact build output
rtk pytest              # only failing tests
rtk kubectl get pods    # compact k8s output
```

## Key Commands

| Command | What it does |
|---------|-------------|
| `rtk git` | Git with compact output |
| `rtk gh` | GitHub CLI, token-optimized |
| `rtk test` / `rtk pytest` / `rtk vitest` | Tests — only show failures |
| `rtk err <cmd>` | Run anything, show only errors/warnings |
| `rtk diff` | Ultra-condensed diff (changed lines only) |
| `rtk json <file>` | Compact JSON, `--schema` for structure only |
| `rtk find` | Compact file tree |
| `rtk grep` | Strips whitespace, truncates, groups by file |
| `rtk curl` | Auto-JSON detection + schema output |
| `rtk summary <cmd>` | Run anything, get 2-line heuristic summary |
| `rtk gain` | Show token savings stats |
| `rtk discover` | Find missed savings from past sessions |

## Global Flags

- `-u, --ultra-compact` — Level 2 compression (ASCII icons, inline format)
- `--skip-env` — Set `SKIP_ENV_VALIDATION=1` for child processes

## Supported Tools (61 commands)

git, gh, docker, kubectl, cargo, npm, npx, pnpm, pip, go, pytest, vitest, playwright, ruff, mypy, tsc, next, lint, prettier, prisma, psql, aws, curl, dotnet, rake, rubocop, rspec, golangci-lint, and more.

## When to use

Prefix ALL Bash commands with `rtk` to save 60-90% tokens. If `rtk` doesn't have a specialized filter for a command, use `rtk err` (errors only) or `rtk summary` (2-line summary) as fallbacks.
