# Changelog

## 0.0.14

### Added

- Support generating shell completion via `bite completion`.

### Changed

- Bump the minimum supported rust version to 1.84.

## 0.0.13

### Added

- Support a configurable concurrent request limit.
- Support service type positional arguments for the `show connections` command.
- Support overriding the system proxy settings via client parameters.
- Add native-tls and rustls-tls features to allow TLS backend choice, defaulting to rustls.

### Changed

- Use visible aliases for subcommands which also allows for generation shell
  completion to work for aliases.
- Bump the minimum supported rust version to 1.80.

## 0.0.12

### Fixed

- Discard output from editor and browser commands when launching them.

### Added

- Support custom root certificate for service connections via --certificate.
- Support loading custom user configs.
- Support loading and saving templates for various actions (e.g. search) into
  user config locations for named connections.

#### Bugzilla

- attachment create: Support unit symbols for --auto-compress value.
- attachment get: Add -D/--deleted and -O/--obsolete options.
- fields: Add initial fields command support.
- search: Add -@ short option for --attachments.
- search: Add support for restricting by bug closure time with --closed.
- search: Add initial -O/--offset support.
- search: Add support for custom field existence queries.
- search: Add support for custom field changed-related queries.
- search: Add initial paged request support.
- search: Add inversion support for changed queries.
- search: Add range support for blocks and depends queries.
- version: Add initial version command support returning the service version.

#### Redmine

- search: Add initial -O/--offset support.
- search: Add initial paged request support.

## 0.0.11

### Changed

- Rework service command layout to make usage patterns more consistent. This includes
  dropping support for linked connection commands, replacing the -s/--service option with
  a required subcommand, and merging the -b/--base option into -c/--connection. See the
  related command documentation for further usage examples.
- Try launching $BROWSER before using xdg-open for web browser support.

#### Bugzilla

- attachment create: Rename `attach` command to `attachment create`.
- attachment create: Drop --dir option to use automatic directory target handling instead.
- attachment create: Rename -s/--summary to -d/--description.
- attachment create: Add -f/--flag option to set attachment flags.
- attachment create: Add -n/--name option to explicitly set file name.
- attachment get: Rename `attachment` command to `attachment get`.
- attachment get: Rename -V/--view option to -o/--output.
- attachment update: Add initial attachment metadata update command support.
- search: Separate match operators from values with a single space.
- search: Support various attachment-related queries such as `--attachment-mime`.
- search: Support comment tag and privacy queries.
- search: Alter match operators to all be two characters long.
- update: Rename `modify` subcommand to `update` for consistency with upstream docs.

## 0.0.10

### Changed

- Move long help for commands into external documentation. Use the man pages or
  online docs for additional information beyond what -h/--help provides.

#### Bugzilla

- search: Use comma-separated values instead of multiple options combined via
  logical OR for bug fields that are unique, e.g. -C/--component values.

## 0.0.9

### Changed

#### Bugzilla
- search: Revert to using comma-separated values for --id and --status.
- search: Drop support for comma-separated time values.
- Use bug ID headers for comment and history command output.

### Added

- Support determining connection via binary symlink.

#### Bugzilla
- attach: Support targeting directories via -d/--dir.
- search: Support using group fields for -f/--fields values.
- search: Support ranges of ID values.
- search: Support static datetime values for relevant options.
- modify: Support standard input for -c/--comment.
- history: Add request parameter support and filtering.

#### Redmine
- search: Support static datetime values for relevant options.

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
