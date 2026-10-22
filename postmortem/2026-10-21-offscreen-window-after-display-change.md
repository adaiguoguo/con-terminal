# Window left off-screen after a display change

## What happened

A user moved a running Con between a home and an office external display. After
the office display was attached, the workspace window sat almost entirely
outside the visible screen area; only its right edge (about 80pt) remained.

## Root cause

In the reported display swap, AppKit left a window at its old frame because it
still overlapped a remaining screen by a sliver. Con did not revalidate that
position after display reconfiguration, so nothing corrected the stale frame.

## Fix

`macos_windowing` observes `NSApplicationDidChangeScreenParametersNotification`.
After 300 ms without another notification it checks visible, titled,
non-fullscreen windows. A window with less than half its area inside the
screens' combined visible frames moves to the screen it overlaps most (the
menu-bar screen when it overlaps none) and shrinks to fit when necessary.
Measuring the union of screen
overlap keeps a window that deliberately spans two displays in place without
double-counting mirrored regions. Borderless windows such as the quick terminal
keep managing their own frame. Recovery neither activates windows nor changes
the first responder.

## Verification

- Unit tests cover the frame recovery decision and geometry, including negative
  display coordinates, overlapping screens and the half-visible threshold.
  Virtual-clock tests cover trailing-edge notification coalescing and
  cancellation on stream closure.
- The contributor's debug build with an isolated HOME was moved to x=-1100,
  then the display mode was switched temporarily. The fixed build returned to
  x=0; a `main` build stayed at x=-1100.
- The contributor confirmed the real home-to-office display swap no longer
  leaves the window off-screen.
- Maintainer validation after updating to current main: all 670 app tests and
  all 27 `con-test` assertions across eight files passed, along with strict
  Clippy, formatting and docs-manifest checks. The live tests used an isolated
  HOME and socket, not the user's running session. They exercise application,
  pane and tab lifecycle; the physical display swap was not repeated by the
  maintainer. Windows/Linux runtime behavior was not tested locally.

## Lessons

Window placement must be revalidated on display reconfiguration, not only when
a window opens. A temporary display-mode switch reliably triggers the screen
parameter notification for local reproduction.

Repeated notifications must restart the settle timer. Waiting only from the
first event can move a window while AppKit is still changing its placement.
