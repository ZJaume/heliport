# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased
### Added
 - CLI tests.
### Changed
 - Sentences without alphabetical characters that cannot be identified are now labeled as `zxx` instead of `und`, which is now only used for predictions under confidence thresholds.

## v0.10.0
### Added
- Add new languages documentation.
- Model singleton shared between Identifier instances for Python API.
- Collapse macrolangs in top-k.

### Changed
- Python API methods refactor.
- Updated dependencies.
- Re-compile model if heliport-model subpackage changed.
- Optional check all langs have threshold before loading.
- Fix confidence file parsing on Windows.
- Other small changes.

### Removed
- Download feature.
- Python 3.8 support.

## v0.9.0
### Added
- More Python identifier functions: `identify_with_score`, `identify_topk_with_score`, `par_identify`, `par_identify_with_score`.
- Batched and multithreaded Python prediction.

### Changed
- Rust identifier functions won't return `Option` anymore, as there was no case where `None` was used.
- Included `Cargo.lock`.
- Update Github Actions build container to Python 3.12.
- Update Github Actions to MacOS 13.
- Update to Maturin 1.8.

### Fixed
- Small documentation issues.

## v0.8.1
### Fixed
- `sdist` project layout without `heliport.data`

## v0.8.0
### Added
- Model creation command.
- More verbosity during identification.

### Changed
- Include binarized model in the wheel.
- Binarize model during compilation.
- Separate languagemodel stuff in a subpackage.
- Disable download feature by default.
- Use Rustls for download instead of OpenSSL (less hassle when building from source).
- Parallelize model binarization.
- Update Python bindings to PyO3 0.23.

### Fixed
- Fix compilation without python feature.
- Min Dong missing as CJK language.
- Removed old entrypoints.

## v0.7.0

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
