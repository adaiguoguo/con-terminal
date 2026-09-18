# Terminal title activity hidden by tab presentation

## What happened

Applications that animate their OSC title were visibly active in Ghostty's
native tabs, but Con's collapsed tab rail kept displaying a static terminal
icon. User and AI tab names could also hide the changing title in expanded
and horizontal tabs.

## Root cause

The `SET_TITLE` ABI was already connected. Con treated the title primarily as
naming context rather than a separate source of transient presentation.
`PROGRESS_REPORT` is an independent protocol and cannot substitute for title
animation. Every title change also entered the tab-summary request path.

## Fix applied

- Cache raw title, display name and source-owned indicator per terminal view,
  using one platform-independent parser. Do not persist activity or run a host
  animation timer.
- Require two different standalone spinner frames with the same body before
  projecting activity. Preserve static Unicode titles. Recognize Codex's
  explicit `Action Required` segment separately from activity.
- Render the current frame in a fixed icon slot, independent of explicit/AI
  names. Aggregate all surfaces, including hidden ones, with attention before
  focused activity and stable tree-order fallback.
- Route frame-only changes through a single sidebar-row update and exclude
  frames from subsequent summary context. Refresh cached presentation when
  splitting/reordering panes; seed rename from the displayed name.

## What we learned

A terminal title is application-owned presentation, not a reliable universal
AI lifecycle protocol. Do not infer activity from output, CPU or process
existence, and do not invent continued motion when the application stops
changing its title. Arbitrary unknown title formats remain raw text.

Review exposed literal Braille false positives, redundant summary triggers on
animation recognition, whitespace loss and leftover Codex separators. The
parser regressions were run failing before their fixes and passing afterward.
The tests include Unicode boundaries, recognition, completion, attention and
10,000 frames with one naming-context change. UI validation should use real
OSC writes through a live PTY in collapsed, expanded and horizontal layouts,
including user labels and hidden surfaces, rather than only parser tests.
