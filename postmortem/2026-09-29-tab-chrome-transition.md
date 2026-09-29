# Tab chrome contrast and close transition

## What happened

After the title bar was aligned with the terminal background and transparency,
ordinary tabs still painted a nearly opaque `theme.background` fill. Against
the translucent title bar, they looked like separate dark blocks. Closing the
penultimate tab also exposed a short, opaque 8 px strip before the remaining
terminal occupied the compact title-bar layout.

## Root cause

The tab fill was chosen for the previous, opaque chrome rather than the current
title-bar surface. On macOS, the top-chrome snap guard deliberately waits for
the embedded native terminal view, then paints a release cover below the new
28 px bar. That cover used the opaque terminal seam color, so it could be seen
as a separate bar over a translucent window.

The report that a newly created tab sometimes stops responding while its title
is summarized is not yet attributable to the model request. The request runs
on the shared background runtime, but sampling recent terminal lines and
capturing the session snapshot run on the UI thread.

## Fix applied

- Use a subtle foreground tint for ordinary tab surfaces so their contrast is
  relative to the title bar in both light and dark themes.
- Match the short top-chrome release cover to the title-bar material rather
  than painting an isolated opaque terminal-colored strip.
- Close tabs on a completed click, stop propagation to tab activation, and
  enlarge the close target. Removing the tab during mouse-down could otherwise
  invalidate the click target before GPUI finished dispatching the gesture.
- Log only slow UI-thread session snapshots and summary sampling so any
  remaining intermittent stall can be assigned to a measured path.

## What we learned

Native terminal views and GPUI chrome do not share a compositor. Temporary
seam covers need to match their visual neighbor as well as their geometry.
Background model inference does not imply all title-summary preparation is
off the UI thread; measure the synchronous work before changing the scheduler.
