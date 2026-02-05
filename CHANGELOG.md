# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning.

## [1.2.0] - 2026-02-05

- Add automatic interface assignment for Kea when using `--create-subnets` (populates listening interfaces).
- Add automatic interface assignment for dnsmasq when using `--create-subnets`.
- Add `--enable-backend` flag to disable ISC DHCP on migrated interfaces and enable the target backend.
- Make ISC DHCP disablement compatible with OPNsense by removing `<enable>` tags instead of writing `0`.
- Guard `--enable-backend` so it refuses to run if ISC DHCP already appears disabled (prevents dual-DHCP on repeat runs).
- Verify now normalizes XML before diffing, avoiding noisy formatting-only diffs.
- Add post-convert verification that ISC DHCP disablement actually took effect.
- Refactor CLI and migrate modules for maintainability.
- Update README scripted usage, add CI/release badges, and document enable-backend workflow notes.

## [1.1.2] - 2026-02-04

- Enforce strict interface validation to prevent assigning reservations to the wrong interface.
- Add interface-aware fixtures and tests to cover mismatch cases for Kea and dnsmasq.
- Update README quick start and testing notes for OPNsense 26.1 configs.

## [1.1.1] - 2026-02-04

- Add DHCP option migration (Kea option_data and dnsmasq `type=set` options).
- Refactor modules and add CLI tests.
- Add developer tooling updates (testing docs and Makefile).

## [1.1.0] - 2026-02-04

- Add dnsmasq backend support.
- Add subnet/range creation from ISC ranges for Kea and dnsmasq.
- Improve docs and tests around subnet/range creation.

## [1.0.1] - 2026-01-22

- Release workflow fixes and packaging updates.
- Dependency lockfile updates.

## [1.0.0] - 2026-01-22

- Initial release.
