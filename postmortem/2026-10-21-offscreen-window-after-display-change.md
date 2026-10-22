# Window left off-screen after a display change

## What happened

A user moved a running Con between a home and an office external display. After
the office display was attached, the workspace window sat almost entirely
outside the visible screen area; only its right edge (about 80pt) remained.

## Root cause

AppKit relocates a window only when the screen it belongs to disappears. A
window that still overlaps a remaining screen by a sliver keeps its frame, even
when the new display arrangement leaves most of it off-screen. Con did not
observe display reconfiguration and does not persist window frames, so nothing
corrected the stale position.

## Fix

`macos_windowing` observes `NSApplicationDidChangeScreenParametersNotification`.
After a short settle delay it checks visible, titled, non-fullscreen windows;
a window with less than half its area inside the screens' combined visible
frames moves to the screen it overlaps most (the menu-bar screen when it
overlaps none) and shrinks to fit when necessary. Summing the overlap keeps a
window that deliberately spans two displays in place. Borderless windows such as the quick terminal
keep managing their own frame.

## Verification

- Unit tests cover the frame recovery decision and geometry.
- A debug build with an isolated HOME was moved to x=-1100 and the display mode
  was switched temporarily. The fixed build returned to x=0; a `main` build
  stayed at x=-1100.
- The user confirmed the real home-to-office display swap no longer leaves the
  window off-screen.

## Lessons

Window placement must be revalidated on display reconfiguration, not only when
a window opens. A temporary display-mode switch reliably triggers the screen
parameter notification for local reproduction.
