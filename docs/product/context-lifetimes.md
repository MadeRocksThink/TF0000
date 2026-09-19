# Context scope and lifetime rules

This page describes the model's intended scopes. Interfaces expose only a subset; [known limitations](../KNOWN_LIMITATIONS.md) describes the current behavior.

## Availability scopes

- `global`: eligible in every project and conversation unless explicitly disabled.
- `project`: eligible only within one project.
- `conversation`: eligible only in one captured conversation.
- `task`: eligible only while the associated task is active.

## Attachment lifetimes

- `one_prompt`: removed after the next successful prompt handoff.
- `n_prompts`: initialized with a positive remaining count and removed at zero.
- `session`: models a session attachment, but automatic session cleanup and time-based expiry must not be assumed in the current implementation; clear it manually.
- `conversation`: remains attached to the selected conversation.
- `manual`: remains until explicitly removed.

Counters are decremented after a successful insertion, never when merely previewing or copying context. They do not observe whether a prompt was actually sent. The `expiresAt` value is not reliably enforced during composition; remove expired material manually.
