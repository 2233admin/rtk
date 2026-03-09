#!/usr/bin/env bash
# rtk-hook-version: 1
# Delegates to `rtk hook-intercept` which reads JSON from stdin.
# To change interception logic, edit src/hook_intercept_cmd.rs — not this file.

if ! command -v rtk &>/dev/null; then exit 0; fi

RTK_VERSION=$(rtk --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
if [ -n "$RTK_VERSION" ]; then
  MAJOR=$(echo "$RTK_VERSION" | cut -d. -f1)
  MINOR=$(echo "$RTK_VERSION" | cut -d. -f2)
  if [ "$MAJOR" -eq 0 ] && [ "$MINOR" -lt 26 ]; then exit 0; fi
fi

rtk hook-intercept 2>/dev/null || exit 0
