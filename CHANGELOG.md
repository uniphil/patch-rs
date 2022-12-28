# CHANGELOG

## [Unreleased]
### Changed

## [v0.7]
### Breaking
- Filename parsing now expects (and renders) a tab character after the filename instead of a space character, before any metadata. Seems like all diff programs actually follow this convention, and git will even render unquoted filenames with spaces, so the previous parsing would produce incorrect results. Thanks [@keith](https://github.com/keith) for reporting.

### Fixed
- Don't break (hopefully) on diffs with windows-style CRLF line endings. I don't have samples to verify with, but it Seems Like Maybe It Will Do The Right Thing? (it will still ony render diffs with line endings as `\n`. Please open a feature request if you want this.) Thanks [@jacobtread](https://github.com/jacobtread) for reporting.
- Parse (and save) hunk hints after range info instead of (incorrectly) treating them like Context lines. Thanks [@keith](https://github.com/keith) and [@wfraser](https://github.com/wfraser).

## [v0.6]
### Changed
- Upgrade nom to 0.7! from [@compiler-errors](https://github.com/compiler-errors)
