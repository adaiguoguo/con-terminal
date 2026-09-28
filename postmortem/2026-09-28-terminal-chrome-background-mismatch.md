# Terminal and chrome background mismatch

## What happened

With `paper-light`, `background-opacity = 0.75`, and UI opacity near 0.90,
the terminal appeared gray over a dark window while the top bar, vertical tab
sidebar, and input bar remained much lighter. These were large stable color
blocks, not the transient native-layout seams investigated in May.

## Root cause

- The title bar used a foreground-tinted theme surface instead of the terminal
  background. The vertical rail and pinned tab list applied their own tints.
- Chrome used a separate opacity curve: UI 0.90 became approximately 0.9565,
  while macOS read the terminal's native 0.75 unchanged.
- Both the input bar and sidebar painted translucent child backgrounds over
  already-painted parents, increasing effective opacity again.
- GPUI added an NSVisualEffectView while Ghostty also applied window blur.
  Matching an RGB triple could not make these different compositions identical.

## Fix applied

The workspace owns each of the three base fills. They use the terminal background
and effective opacity; sidebar and input-bar children do not repaint a base.
Independent panels, selections, and popups retain their UI-opacity setting.
The input bar's static opaque separator is replaced by transparent padding:
over a black backdrop that separator otherwise becomes a bright horizontal line.
Transient native-layout seam guards remain separate and unchanged. Redundant
portable workspace mattes are removed too; terminal views own their own fill.

The macOS window remains transparent without a second GPUI blur material. Native
window blur reads Ghostty's resolved value, including includes and legacy aliases.
Zero is applied explicitly when blur is disabled or opacity becomes one. Liquid
Glass materials are unsupported by the host and produce a warning with unblurred
transparency, rather than passing negative sentinels as blur radii.

## What we learned

Trace parent and child paint layers before introducing more synchronized state.
Central ownership of these fills removes both duplicate composition and the need
to propagate opacity into cached child entities. Keep terminal backing clear;
never solve this by adding a full-terminal opaque matte.

Use the exact Ghostty C ABI, not a guessed integer width: `background-blur` writes
an `i16`. A native-parser test caught an initial `i32` destination reading the
regular-glass sentinel as 65535 instead of -1. Regression tests cover theme color
identity, included blur values, both glass sentinels, and legacy-alias precedence.
