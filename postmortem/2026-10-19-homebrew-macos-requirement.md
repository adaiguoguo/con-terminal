# Generated Homebrew casks rejected by current Homebrew

## What Happened

The published beta.141 cask still contained
`depends_on macos: ">= :catalina"`. Homebrew rejected it before installation
could begin. A successful Con release or a syntactically valid Ruby file did
not establish that Homebrew could load the cask.

## Root Cause

The macOS release workflow regenerated a versioned requirement using an old
macOS symbol. Current Homebrew no longer recognizes `:catalina`; it also
deprecates the string comparison form. The existing packaging fixtures checked
shell scripts and bundle handoff, not the generated Homebrew DSL.

## Fix

[PR #494](https://github.com/nowledge-co/con-terminal/pull/494) by
[@colmugx](https://github.com/colmugx) changes the generator to
`depends_on :macos`. This retains its macOS-only restriction without tying it
to a removed release symbol. It changes neither application rendering nor the
binary deployment target, and does not modify Linux or Windows packaging.

The follow-up fixture extracts the actual workflow template and loads it with
the real Homebrew cask loader. It checks beta/stable and ARM/Intel metadata,
including the selected URL and checksum. The macOS packaging CI runs it when
the generator or packaging tests change.

## Verification and Limits

Local verification used Homebrew 7.0.9-129-g19fe946. The original requirement
raised an unsupported-Catalina error; generated casks with the new requirement
loaded in all four channel/architecture cases. Architecture simulation here
only evaluates metadata; no cross-compilation or foreign binary execution is
involved. No application was installed or upgraded, and no tap was changed.
The next release must regenerate the published tap entry; merging the generator
fix alone does not rewrite an already-published cask.

## Lesson

Validate release-generated files with their consuming tool, not only a syntax
checker. A cask loader can reject a dependency even when packaging, signing and
the app's own tests all pass.
