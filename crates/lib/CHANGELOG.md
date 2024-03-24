# Changelog

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
