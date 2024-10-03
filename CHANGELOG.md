# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Faster `score\_lang` triggering autovectorization.
- Multithreaded identification.
- Load only relevant languages feature.
- More verbosity when binarizing models
- Support i/o files other than stdin/stdout.
- Rank top k languages (still not available in the CLI).
- Alias `detect` comand for `identify`.
- More verbosity when binarizing models
- MSRV to 1.71.

### Changed
- Bring back HeLI models (use 2.0).
- Refactored the CLI into subcommands
- Renamed `compile` command to `binarize`
- Better error propagation with Anyhow.
- Separate CLI, download and python into crate features.
- Renamed Model to ModelNgram and Models to Model.
- Update tests.
