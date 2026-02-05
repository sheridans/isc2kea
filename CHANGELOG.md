# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning.

## [1.1.3] - 2026-02-05

- Add automatic interface assignment for Kea when using `--create-subnets` (populates listening interfaces).
- Add automatic interface assignment for dnsmasq when using `--create-subnets`.
- Add `--enable-backend` flag to disable ISC DHCP on migrated interfaces and enable the target backend.

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
