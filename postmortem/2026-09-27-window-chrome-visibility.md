# Window chrome visibility when reopening Con

## What happened

I noticed that closing the last macOS window and reopening Con reset the visibility of the left sidebar, input bar, and agent panel. I would appreciate confirmation that this reset is not intentional; this change proposes retaining those choices when creating a fresh window.

## Root cause

The toggle actions already save all three fields in Session, and a normal cold start restores them. On macOS, closing the last window leaves the process alive. Reopening uses `fresh_window_session_with_history_for_cwd`, which starts from Session defaults and copies history without copying visibility. New windows and launches with a working directory or command also use this helper.

## Proposed fix

Copy `input_bar_visible`, `left_panel_open`, and `agent_panel_open` from the saved session when creating a fresh window. Existing fresh-terminal, history, and working-directory behavior is preserved.

## Validation

- Two unit tests pass, covering all eight visibility combinations, defaults, fresh tabs, the requested working directory, and history precedence.
- Formatting and `git diff --check` pass.
- I tested a local macOS build with separate session, history, and socket paths using real menus, shortcuts, and the window close button.
- With sidebar and input bar hidden and agent panel visible, both closing/reopening the window and quitting with Cmd+Q/relaunching retained the settings.
- With all three hidden, both paths retained the settings.
- I checked screenshots and saved Session fields, and checked agent panel visibility through the control API. Process or socket checks confirmed full termination before relaunch.
- The pre-change binary reproduced the reset: after closing/reopening, sidebar/input/agent visibility changed from false/false/true to true/true/false, confirmed on screen and in Session.
- The test instance was closed afterward; the installed Con Beta was not replaced.

## What we learned

Cold-start restoration and fresh-window creation need separate coverage. If retaining visibility is the intended design, fresh windows need to carry those choices independently of terminal session contents.
