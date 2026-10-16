# Agent tab icon switched on brand names in chat output

## What happened

On macOS (Beta 0.1.0, build 144) a tab running one agent CLI switched its
brand icon to other agents' icons several times while chat and tool output
updated. The foreground program never changed.

## Root cause

`refresh_terminal_presentation` resolves a surface's agent from three sources:
native foreground process name, OSC title, then visible screen text. Script
agents (Gemini CLI, npm Claude Code, Cline, Kimi, ...) run as `node`/`python`,
so neither process nor title identifies them and the screen scan decides.

The reporter's tabs ran native `claude` and `codex`, which normally resolve
by process name. Their login shell was wrapped by Kiro CLI's shell
integration (`kiro-cli-term`, sourced from `.zprofile`/`.zshrc`), which runs
the real shell on an inner PTY. Con's PTY foreground group therefore held
only `login` and `zsh (kiro-cli-term)`; the agent lived on the inner TTY:

```
con Beta → login (ttys004, fg group) → zsh (kiro-cli-term)
         → [inner ttys005] zsh → claude
```

The wrapper's name matches no agent, so process identity was never
available and every tab fell through to the screen scan below, where the
chat text decided the icon (Codex, Kimi, ...).

- Every input generation changes `AgentCliObservation`, which reopened the
  screen-scan budget. Each rescan replaced the previous identity with the first
  brand string found in 200 lines, in a fixed priority order (`openai codex`,
  `claude code`, then `agent_from_screen_text`). Chat, quoted code and tool
  logs mentioning another product therefore relabeled the tab.
- The scan read the whole viewport, including output left by earlier shell
  commands. Gemini CLI paints an ASCII-art banner with no brand text, so a
  shell line mentioning "OpenAI Codex" above it named the tab Codex (observed
  live in a debug build).
- Screen text was also scanned for any non-shell foreground process, and while
  process evidence was unavailable. A pager/editor showing a README, or the
  window between a job change and its process query, could adopt the old
  banner; once the scan budget finished, that stale identity stayed.

## Fix

`Surface::resolve_identity` keeps the precedence native process > title >
screen, and constrains screen evidence:

- Screen text may only name an interpreter host (`node`, `bun`, `deno`,
  `python*`) found in the foreground processes. Unknown or unavailable
  processes and non-interpreters get no screen identity.
- The host process identity is tracked stably across unrelated child churn.
  Its change bumps the identity generation and reopens detection.
- A screen identity confirmed for the current generation is kept without
  rescanning, so later text cannot replace it. Exiting the host, a native
  agent appearing, or a shell foreground clears it.
- Lines visible before the program started are subtracted (by count) from the
  scan. The snapshot refreshes on typing at the prompt and once when a new
  foreground job starts, before its process list arrives, because PTY writes
  from `con-cli` or the built-in agent do not advance the input generation.
- On Windows, a process tree containing only shells also refreshes this
  baseline and clears stale title evidence. Ancestry still does not establish
  foreground ownership when other processes remain in the tree.
- Exact native process names for Qoder, Hermes, Cline, Qwen and Kilo bypass
  screen inference, preserving their icons after the fallback is narrowed.

## Verification

Unit tests in `terminal_status.rs` cover chat mentions, non-interpreter and
unavailable processes, program switches, shell return, shell residue and PTY
writes. A live debug build (isolated `HOME` and socket) confirmed: native
Claude and npm Codex (node wrapper plus native `codex` in one process group)
keep their icons while the other product is typed into their input; the shell
restores the terminal icon; Gemini after Codex-mentioning shell output shows
the terminal icon instead of Codex. Windows/Linux desktop behavior was not
tested locally. Their portable CI now runs the terminal-status and tab-identity
tests, including shell-only tree transitions, old-banner isolation and native
agent precedence. A retained-identity test rejects any further screen read
after confirmation, including when input changes.

During maintainer verification, both macOS E2E attempts on application commit
`4bd04e30` exhausted the 30-minute job budget while building, before any E2E
test ran. Zig provisioning took about eight minutes in each attempt. The job
budget was increased to 60 minutes for cold builds; the tests and their
30-second application-startup limit were not changed. Windows and Linux
portable CI passed on that application commit, including the new tests.

Behind a PTY wrapper such as `kiro-cli-term` (or tmux/ssh) the agent is not
in Con's foreground group, so the tab now shows the terminal icon instead of
a guessed brand. Following a wrapper's inner TTY to its foreground group
would restore the brand there; it is not part of this fix. The reporter
removed the Kiro shell integration, which restores process identity.

## Lessons

Screen text is an untrusted, content-derived signal. It should only label an
otherwise anonymous process once per process lifetime, never arbitrate between
brands on every repaint.

Check the shell's process ancestry before reproducing an identity bug: a
test session that inherits the reporter's environment (here `Q_TERM`) can
skip the wrapper and hide the real trigger.
