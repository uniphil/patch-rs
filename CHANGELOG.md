# CHANGELOG

## [Unreleased]
### Breaking change
- Filename parsing now expects (and renders) a tab character after the filename instead of a space character, before any metadata. Seems like all diff programs actually follow this convention, and git will even render unquoted filenames with spaces, so the previous parsing would produce incorrect results.

## [v0.6]
### Changed
- Upgrade nom to 0.7! from [@compiler-errors](https://github.com/compiler-errors)

