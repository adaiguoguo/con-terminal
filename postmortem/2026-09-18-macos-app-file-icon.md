# macOS app icon changed only in the running Dock

## What happened

Saving an alternate raccoon icon changed the running Dock / Cmd-Tab image,
but Finder and application search continued to show the bundled Classic icon.

## Root cause

`NSApplication.applicationIconImage` changes the running process's image,
not the installed application's file icon. The saved configuration and
preview stack worked, but there was no `NSWorkspace.setIcon` path.

## Fix applied

Keep previews process-local. After successful config persistence, and when
loading config on launch or reopen, synchronize the main `.app` file icon
using `NSWorkspace.setIcon`. Classic removes the custom icon with `nil`.
Non-bundle development launches do not customize their containing directory.

Use GPUI's existing background executor: a native probe took about 160–177 ms
for an initial update, too long for the UI thread. Serialize writes, read the
latest saved selection after locking, and cache only successful updates.
Failures are logged and can be retried on the next save or launch. No Dock
plugin, new dependency, timer, cache purge, or signed-resource replacement
is required. Native file-icon reads observed changes without the deprecated
`noteFileSystemChanged` notification.

## What we learned

Runtime and file icons need separate state: a Dock preview must not suppress
the later file update. Tests cover previews, persisted choices, failed-write
retry, duplicate suppression, queued saves, and native set/reset on a disposable
directory. Native rendering of Classic after reset matched the original bytes.

Post-install custom icons add Finder metadata. Ordinary signature verification
passed on an ad-hoc-signed test bundle, while `codesign --strict` rejected the
custom metadata; resetting Classic restored strict verification. Release
artifacts must remain pristine and strictly verified. Developer-ID notarized
launch and a real Sparkle update require separate release acceptance testing.
