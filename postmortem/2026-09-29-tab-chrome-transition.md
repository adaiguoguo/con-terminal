# Tab chrome contrast and close transition

## What happened

After the title bar was aligned with the terminal background and transparency,
ordinary tabs still painted a nearly opaque `theme.background` fill. Against
the translucent title bar, they looked like separate dark blocks. Closing the
penultimate tab also exposed a short, opaque 8 px strip before the remaining
terminal occupied the compact title-bar layout.

## Root cause

The tab fill was chosen for the previous, opaque chrome rather than the current
title-bar surface. More importantly, the macOS title bar changed height from
36 pt to 28 pt when the penultimate tab closed. That resized the embedded
Ghostty NSView while GPUI and AppKit committed their frames independently. A
timed snap guard and release cover tried to hide the gap; matching the cover's
color to the title bar reduced the contrast but could not eliminate it.

There was also a shared motion-state bug: a zero-duration transition still
reported itself as animating until the next frame. Terminal-adjacent chrome on
macOS uses zero-duration transitions, so this false state could activate seam
handling even though no animation should occur.

The report that a newly created tab sometimes stops responding while its title
is summarized is not yet attributable to the model request. The request runs
on the shared background runtime, but sampling recent terminal lines and
capturing the session snapshot run on the UI thread.

## Fix applied

- Use a subtle foreground tint for ordinary tab surfaces so their contrast is
  relative to the title bar in both light and dark themes.
- Keep the macOS top bar at 28 pt with either one or multiple tabs. The tab
  strip now fits within that same area, so a tab-count change does not resize
  the native terminal or need a top-chrome snap guard and release cover.
- Settle zero-duration motion synchronously. It no longer reports an extra
  transition frame or activates transition-only seam handling.
- Close tabs on a completed click, stop propagation to tab activation, and
  enlarge the close target. Removing the tab during mouse-down could otherwise
  invalidate the click target before GPUI finished dispatching the gesture.
- Log only slow UI-thread session snapshots and summary sampling so any
  remaining intermittent stall can be assigned to a measured path.

## What we learned

Native terminal views and GPUI chrome do not share a compositor. When a chrome
state change need not alter native terminal bounds, keep those bounds fixed
instead of masking a transient mismatch with a timed cover. Other chrome
changes that genuinely resize terminal content need separate, measured work;
this fix does not remove their native-layout guards.
Background model inference does not imply all title-summary preparation is
off the UI thread; measure the synchronous work before changing the scheduler.
