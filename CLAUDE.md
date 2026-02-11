# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LED Sectional Rust Conversion targeting Raspberry Pi and ESP32 platforms. This is an embedded Rust project for controlling LED strips/arrays.

## Development Environment

Managed via [mise](https://mise.jdx.dev/). Run `mise install` to set up:
- **Rust 1.92**
- shellcheck (shell script linting)
- shfmt (shell script formatting)

## Build Commands

Once Cargo.toml is created:
- `cargo build` — build the project
- `cargo test` — run all tests
- `cargo test <test_name>` — run a single test
- `cargo clippy` — lint
- `cargo fmt` — format code
- `cargo fmt --check` — check formatting without modifying

## Architecture

Project is in initial setup phase. No source code exists yet. The codebase targets two embedded platforms (Raspberry Pi and ESP32), which will likely require platform-specific HAL (Hardware Abstraction Layer) code and possibly a Cargo workspace or feature flags to support both targets.
