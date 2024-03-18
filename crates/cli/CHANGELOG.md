# Changelog

## 0.0.4

### Added

#### Bugzilla
- Support creating and modifying bugs via `create` and `modify`.

## 0.0.3

### Added

#### Bugzilla
- attach: Support pulling bug IDs from stdin.
- get: Add support for the keywords, platform, deadline, and OS bug fields.
- search: Add support for the alias, keywords, depends, blocks, and cc fields.
- search: Support ordering search results by resolution, depends, and deadline.
- search: Drop assigned-to from default fields.
- search: Drop support for searching aliases.
- search: Rename -S/--sort option to -o/--order.
- search: Support `+` prefixes for ascending order with search order values.
- search: Support initial comment content searching.

## 0.0.2

### Added

- Support targeting connections via subcommand: this allows using commands such
  as `bite linux search -c 1d` where `linux` is a connection alias.

#### Bugzilla
- Support creating attach requests with MIME type auto-detection.

#### Redmine
- Initial support for `get` and `search` requests.

## 0.0.1

- Initial release supporting read-only operations via Bugzilla's REST v1 API.
