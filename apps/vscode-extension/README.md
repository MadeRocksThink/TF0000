# TF0000 for VS Code

Use the same local TF0000 context across VS Code, GitHub Copilot, ChatGPT, Claude, and Gemini.

1. Start TF0000 at least once so its local database exists.
2. Open a repository and run **TF0000: Map Workspace to Project**.
3. Save selected code from the editor context menu or save a file from the Explorer menu.
4. Use the TF0000 sidebar for current decisions, saved references, and pending agent candidates.
5. In Copilot agent mode, enable or reference `#tf0000ProjectContext`, `#tf0000Decisions`, or `#tf0000Remember`.

Read tools never alter memory. The remember tool creates a pending candidate only. Accepting a candidate requires an explicit modal confirmation and creates a draft `ai_suggestion` memory.

The extension does not read Copilot's internal chat transcript. Tools receive only their declared inputs and the current mapped workspace identity.
