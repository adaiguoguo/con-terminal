# Linux mouse reports depended on SGR mode

## What happened

Linux terminal clicks and left-button drags were reported only when the child
enabled SGR 1006. Applications requesting legacy, UTF-8, URXVT, or SGR-pixel
reports received no input through that path. Motion without a held button,
middle-button input, wheel reports, and Ctrl/Alt modifiers were not forwarded.

## Root cause

The Linux backend constructed SGR strings itself and rejected other formats.
The view reduced pointer positions to cell coordinates before dispatch, losing
the sub-cell position needed for pixel reporting. The shared VT layer already
owned an upstream mouse encoder but Linux did not use it.

## Fix applied

Forward normalized physical grid coordinates and mouse modifiers through the
Linux session to the shared encoder. Let Ghostty select the effective protocol,
filter requested event types, and deduplicate motion. Keep gesture ownership in
the view so Shift starts local selection and captured releases still arrive
when modifiers change. Release reports use the existing reserved PTY queue
capacity. Like Windows, emit one directional wheel press per host event and
axis, without an acceleration-dependent burst or a separate accumulator.

## What we learned

Protocol encoding belongs beside the terminal parser, not in platform views.
Pixel coordinates must survive until the encoder chooses the requested format.
Test the host integration as well as the shared encoder: a PTY byte-readback
test catches a platform-specific protocol gate that encoder-only tests miss.
Local scrollback gestures and alternate-screen wheel-to-arrow fallback remain
separate work; this change only reports wheel input to applications requesting it.
