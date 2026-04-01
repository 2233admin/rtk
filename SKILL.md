---
name: slash-tokens
description: CLI proxy that reduces LLM token consumption by 60-90%. Prefix any dev command with 'rtk' to get filtered, compact output. Use for all Bash commands to save context window.
---

# Slash Tokens — 你的 AI 编程账单砍九成

## 故事

有个哥们用 Claude Code 写代码，一个月烧了 $200。他不理解——明明就改了几个文件，钱去哪了？

他翻了翻对话记录，发现了一个惊人的事实：**80% 的 token 不是他写的代码，是工具输出。**

`git log` 吐了 200 行，Claude 只需要看最后 5 行。`npm install` 输出了 800 行进度条，Claude 一行都不需要。`docker ps` 带了一堆格式化边框，全是废话。

这些垃圾输出塞满了 context window，Claude 被迫"阅读"每一行，然后 Anthropic 按 token 收费。

就好比你请了个律师，按小时收费，然后你让他坐在那儿读你的垃圾邮件。

**于是他写了 rtk。**

一个 Rust 写的 CLI 代理，4MB，零依赖。在命令前面加个 `rtk`，输出就被过滤成 AI 真正需要看的部分。

`git status` 吐 30 行？`rtk git status` 给你 3 行。
`npm test` 输出 500 行？`rtk test` 只给你失败的那 2 个。
`kubectl get pods` 带一堆表格边框？`rtk kubectl get pods` 干干净净。

**结果：月账单从 $200 掉到 $40。省了 80%。**

那个哥们后来把 rtk 开源了。现在支持 61 个命令，覆盖你日常用的所有工具。

## Install

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/2233admin/rtk/main/install.sh | bash

# Windows (scoop)
scoop install rtk

# ClawHub (skill only, binary installed separately)
clawhub install slash-tokens
```

## Core Usage

在命令前加 `rtk`，完事：

```bash
rtk git status          # 30 行 → 3 行
rtk git log --oneline   # 去掉装饰符
rtk gh pr list          # token 友好的 PR 列表
rtk npm test            # 只看失败
rtk docker ps           # 紧凑容器列表
rtk pytest              # 只看红的
rtk kubectl get pods    # 去掉表格边框
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

## Supported Tools (61 commands)

git, gh, docker, kubectl, cargo, npm, npx, pnpm, pip, go, pytest, vitest, playwright, ruff, mypy, tsc, next, lint, prettier, prisma, psql, aws, curl, dotnet, rake, rubocop, rspec, golangci-lint, and more.

## When to use

Prefix ALL Bash commands with `rtk` to save 60-90% tokens. If `rtk` doesn't have a specialized filter for a command, use `rtk err` (errors only) or `rtk summary` (2-line summary) as fallbacks.
