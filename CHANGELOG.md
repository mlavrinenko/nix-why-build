# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `cache::Substituters` asks binary caches, in parallel, whether they serve a
  store path, and counts a narinfo only when a trusted key signed it.
- `walk::walk` reads `.drv` files with `nix-compat` and walks from a toplevel
  down to what substituters serve, asking only about the outputs a parent
  wants.
