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

Review of the replacement found two host-side assumptions the encoder cannot
repair: initial presses in padding were clamped into edge-cell clicks, and
GPUI Wayland's single pressed-button slot was treated as the complete button
state. Releasing one button in a chord clears that slot even while another is
held, so a subsequent motion incorrectly ended the remaining capture.

## Fix applied

Forward normalized physical grid coordinates and mouse modifiers through the
Linux session to the shared encoder. Let Ghostty select the effective protocol,
filter requested event types, and deduplicate motion. Keep gesture ownership in
the view so Shift starts local selection and captured releases still arrive
when modifiers change. Release reports use the existing reserved PTY queue
capacity. Like Windows, emit one directional wheel press per host event and
axis, without an acceleration-dependent burst or a separate accumulator.

Reject initial presses and wheel events outside the grid, while retaining
out-of-grid motion and release for captured gestures. Use the existing
per-button captures when a move has no button; only the matching mouse-up or
focus cancellation ends them. No additional button-state container is needed.

## What we learned

Protocol encoding belongs beside the terminal parser, not in platform views.
Pixel coordinates must survive until the encoder chooses the requested format.
Test the host integration as well as the shared encoder: a PTY byte-readback
test catches a platform-specific protocol gate that encoder-only tests miss.
Exercise padding at fractional scale and chord release sequences in the view
with a real PTY too; protocol tests alone cannot expose host routing mistakes.
Local scrollback gestures and alternate-screen wheel-to-arrow fallback remain
separate work; this change only reports wheel input to applications requesting it.
