# DEC2026 frames leaked during synchronized output

## What happened

The Linux and Windows VT renderers could display a partially drawn terminal
while an application held synchronized output (DEC private mode 2026). A
single PTY read can contain completed output, DECSET, and hidden drawing;
checking the mode only after that read cannot recover the intended frame.

## Root cause

The wrapper incremented its generation after each complete parser write and
unconditionally updated its native render state when taking a snapshot. It
had no parser-boundary callback. Keeping the previous rendered frame would
also be incorrect: it might predate completed output, and several release /
start boundaries can occur between draws.

Native `begin_update` consumes terminal damage. Discarding a boundary
capture without rebuilding the canonical native state can therefore lose
rows even if the Rust renderer requests a full redraw. Search highlights,
Kitty placements, cursor state, and viewport metadata must also describe the
captured frame, not the terminal after parsing the rest of the read.

## Fix applied

- Pin the minimal upstream revision with `RENDER_HOLD`. The public-header
  diff adds option 41 and its callback typedef plus lifecycle documentation;
  existing public layouts and signatures, including `ghostty.h`, are unchanged.
- Capture native render state and terminal-owned metadata in the callback.
  Transfer the latest capture to the canonical renderer and finish native
  extraction outside the parser lock. A serial tracks consumed damage so
  superseded or unadopted captures cannot leave stale canonical rows.
  A failed capture retains the previous complete snapshot until release;
  damage recovery must not erase that fallback while the hold is active.
- Transfer search ownership under the search lock during parser writes, and
  reconcile and project its pins at the same boundary. Callback userdata
  remains a shared reference with interior synchronization, not an aliased
  mutable reference to the parser wrapper.
- Keep held generations stable and reserve new generations for every new
  boundary. Renderer acknowledgments cannot clean an unadopted capture.
- Make Windows scrollbar chrome read the snapshot's generation and viewport
  together instead of combining held cells with a newer live scrollbar.
- Arm an independent one-second UI deadline, including for hidden or
  nonblinking cursors. Arm from output wakes as well as rendering so hidden
  Windows panes do not retain a hold indefinitely. Expiration checks the
  original deadline, clears native mode 2026 and the retained hold, advances
  generation, and wakes rendering. Repeated SET does not extend a hold;
  reset and resize release it.

## What we learned

Frame synchronization belongs at the parser boundary, not the polling or
paint boundary. Capture every terminal-dependent datum together; heavy
render extraction and snapshot cloning can then run without the parser
lock. Damage consumption is an ownership transition, not just a dirty bit.

The exact-baseline Linux regression displayed `h` from the hidden suffix
instead of a blank cell. The fixed Linux suite covers same-write boundaries,
coalescing, acknowledgments, discarded capture damage, timeout and wake,
reset/resize, parser concurrency, search, Kitty graphics, and viewport state.
Removing native-state recreation reproduced a missing changed row; moving
the cursor away was essential because cursor damage initially masked that
failure. Removing search reconciliation reproduced a stale highlight over
replacement text. Redundant pre-adoption allocation and invalidation were
removed while keeping the suite green.

Portable VT tests must run on Linux or Windows: a macOS Cargo test filter
can succeed with zero matching tests because that module is cfg-excluded.
The new pin passed the native macOS embedding patch / C ABI build with
`CON_REQUIRE_GHOSTTY_INITIAL_OUTPUT=1`, 24 `con-ghostty` tests, and
`cargo check -p con --locked`. The Windows ARM64 `con-ghostty` type check
passed with native linking skipped. Full Windows application checking was
blocked by missing MSVC C headers on the macOS host; Windows runtime and
Linux/Windows visual verification remain unverified.
