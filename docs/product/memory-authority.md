# Memory authority and status

Authority and lifecycle are separate. Authority describes who asserted content; status describes whether it should be used now.

## Authority

| Value | Meaning |
|---|---|
| `user_confirmed` | Explicitly confirmed by the user; highest default precedence. |
| `external_fact` | Imported fact with preserved source evidence. |
| `ai_suggestion` | Model-generated proposal; never promoted automatically. |
| `inferred` | Derived from evidence but not explicitly confirmed. |

## Status

| Value | Meaning |
|---|---|
| `active` | Eligible for retrieval and attachment. |
| `draft` | Work in progress; visible but excluded by default. |
| `superseded` | Replaced by a later decision/version. |
| `rejected` | Explicitly rejected; retained for provenance. |
| `archived` | Kept for history but excluded from ordinary use. |

The system must never silently change authority. Important replacements require an explicit update action and create a new immutable version.

