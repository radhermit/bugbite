# Changelog

## 0.0.12

### Added

- Support custom root certificate for service connections.
- Support loading custom user configs.
- Support loading and saving templates for various actions (e.g. search) into
  user config locations for named connections.

#### Bugzilla

- fields: Add initial fields request support
- search: Add support for restricting by bug closure time.
- search: Add initial offset support.
- search: Add support for custom field existence queries.
- search: Add support for custom field changed-related queries.
- search: Add initial paged request support.
- search: Add inversion support for changed queries.
- search: Add range support for blocks and depends queries.
- version: Add initial version request support returning the service version.

#### Redmine

- search: Add initial offset support.
- search: Add initial paged request support.

## 0.0.11

### Changed

- Rework service support to expose Request objects allowing combinator-style
  parameter mutation.

#### Bugzilla

- search: Separate match operators from values with a single space.
- search: Support various attachment-related queries.
- search: Support comment tag and privacy queries.
- search: Alter match operators to all be two characters long.

## 0.0.10

### Changed

#### Bugzilla

- search: Re-add explicit case-insensitive substring match operator.
- search: Revert to using `!` instead of `~` for logical NOT.

### Fixed

#### Bugzilla

- attach: Respect explicit MIME type for attachments.

## 0.0.9

### Changed

#### Bugzilla
- Deserialize attachments into temporary files instead of directly into memory.
- search: Use ~ instead of ! for inversion operators.

### Added

#### Bugzilla
- attach: Support targeting directories for attachment creation.
- attach: Add lzip compression support.
- search: Support ranges of ID values.
- search: Support static datetime values for relevant options.
- history: Add request parameter support and filtering.

#### Redmine
- search: Support static datetime values for relevant options.

## 0.0.8

### Added

#### Bugzilla
- attach: Support attachment compression.
- attach: Support auto-compress and auto-truncate options.
- search: Add support for logical OR and AND combinations.
- search: Support loading and saving search parameters using templates.

#### Redmine
- search: Support loading and saving search parameters using templates.

## 0.0.7

### Added

#### Bugzilla
- search: Support time ranges for relevant fields (#16).
- search: Support querying QA contact existence.
- search: Support querying personal tags existence.
- search: Support ranges for change-related options.
- search: Support inverted status matches via `!` prefix.
- search: Replace user alias `@me` for cc and assignee fields.

#### Redmine
- search: Support time ranges for relevant fields (#16).
- search: Support multiple summary terms.
- search: Support querying for blockers, dependencies, and related issues.
- search: Support querying for closed time.
- search: Support querying for attachment existence.
- search: Support querying by assignee.
- search: Support custom query ordering.

## 0.0.6

### Added

#### Bugzilla
- Support using bug aliases in addition to IDs where possible.
- Support creating, altering, and searching by bug flags.
- Support searching by personal bug tags.

- attachment: Support pulling attachments from bug aliases.
- comment: Add initial support for comment filtering.
- modify: Support adding/removing see-also URLs by bug ID.
- modify: Support modifying comment privacy.
- modify: Support modifying aliases.
- search: Support inverted blocker and dependencies queries.
- search: Support various change-related restrictions.
- search: Support match values for keywords, assignees, reporters, urls, and flags.
- search: Support matching against flag setters.
- search: Support matching against aliases.
- search: Support matching against attachment creators.
- search: Support matching against attachment description or filename.

#### Redmine
- search: Add query limit support.
- search: Support status aliases such as @open similar to bugzilla.

## 0.0.4

### Added

#### Bugzilla
- Support creating and modifying bugs.

## 0.0.3

### Added

#### Bugzilla
- Add support for the keywords, platform, deadline, and OS bug fields.
- Add support for the alias, keywords, depends, blocks, and cc fields for searches.
- Support ordering search results by resolution, depends, and deadline.

## 0.0.2

### Added

#### Bugzilla
- Support creating attach requests with MIME type auto-detection.

#### Redmine
- Initial support for `get` and `search` requests.

## 0.0.1

- Initial release supporting read-only operations via Bugzilla's REST v1 API.
