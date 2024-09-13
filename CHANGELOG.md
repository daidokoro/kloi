# Changelog
## [1.0.2-beta] - 2024-09-02
**Added**
- Added interactive mode for stack selection to `check` command
- Added support to used `cfn-lint` to `check` command if present on host

**Fixed**
- Fixed process exit code for errors. Now the process will exit with a non-zero code if an error occurs
- Fixed sdk error handling in `check` command

## [1.0.1-beta] - 2024-09-02
**Added**
- Added interactive mode for stack selection to `apply`, `delete`, `show` commands
- Added `--all/-A` flag to `apply` and `delete` commands to select all stacks

**Updated**
- Updated aws_sdk crate verisons

## [1.0.0-beta] - 2024-08-24
**Added**
- initial release

**Updated**
- initial release

**Fixed**
- initial release
