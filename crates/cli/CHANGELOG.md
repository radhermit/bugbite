# Changelog

## 0.0.8

### Added

#### Bugzilla
- attach: Support attachment compression.
- attach: Support auto-compress and auto-truncate options.
- search: Support logical OR and AND combinations.
- search: Support loading and saving search parameters using templates.
- search: Support JSON output via --json.

#### Redmine
- search: Add --dry-run/-n support.
- search: Support loading and saving search parameters using templates.
- search: Support JSON output via --json.

## 0.0.7

### Added

#### Bugzilla
- search: Support time ranges for relevant fields (#16).
- search: Support opening the query in the browser via -b/--browser.
- search: Support querying QA contact existence.
- search: Support querying personal tags existence.
- search: Support comma-separated IDs for the --id option.
- search: Support range values using operators, e.g. ">=4" for numeric values.
- create: Replace --assigned-to with --assignee.
- modify: Replace --assigned-to with --assignee.

#### Redmine
- search: Support time ranges for relevant fields (#16).
- search: Support opening the query in the browser via -b/--browser.
- search: Support comma-separated IDs for the --id option.
- search: Support multiple summary terms.
- search: Support querying for blockers, dependencies, and related issues.
- search: Support querying for closed time.
- search: Support querying for attachment existence.
- search: Support querying by assignee.
- search: Support custom query ordering.
- search: Support range values using operators, e.g. ">=4" for numeric values.

## 0.0.6

### Added

#### Bugzilla
- Support using bug aliases in addition to IDs where possible.
- Only output attachment ID and summary by default with other relevant info
  being displayed at the info verbosity level.
- Support displaying, creating, altering, and searching by bug flags.
- Support displaying and searching by personal bug tags.

- attachment: Support pulling attachments from bug aliases.
- comment: Add initial support for comment filtering.
- create: Support pre-populating fields using an existing bug.
- modify: Support adding/removing see-also URLs by bug ID.
- modify: Support modifying comment privacy.
- modify: Support pulling the last comment for -R/--reply with no args.
- modify: Support modifying aliases.
- search: Support inverted blocker and dependencies queries.
- search: Support various change-related restrictions.
- search: Support match values for keywords, assignees, reporters, urls, and flags.
- search: Support matching against flag setters.
- search: Support matching against aliases.
- search: Support matching against attachment creators.
- search: Support matching against attachment description or filename.

#### Redmine
- search: Add -l/--limit support.
- search: Support status aliases such as @open similar to bugzilla.

## 0.0.5

### Fixed

#### Bugzilla
- modify: Fixed overlapping short options for --reply and --resolution.

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
