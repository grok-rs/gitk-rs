# Test Fixtures

This directory contains test data and fixtures for gitk-rs integration tests.

## Structure

- `sample_repos/` - Sample Git repositories for testing
- `config_files/` - Sample configuration files
- `diff_samples/` - Sample diff outputs for testing diff parsing
- `mock_data/` - Mock data for testing various components

## Usage

These fixtures are used by integration tests to ensure consistent and reliable testing across different scenarios.

Test fixtures should be:
- Minimal but comprehensive
- Deterministic and reproducible
- Representative of real-world data
- Well-documented

## Creating New Fixtures

When adding new test fixtures:

1. Keep them as small as possible while still being useful
2. Document what they test and why they're needed
3. Ensure they work across different platforms
4. Use descriptive names that explain their purpose