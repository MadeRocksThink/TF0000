# TF0000 browser extension

> This integration is a preview. See [known limitations](KNOWN_LIMITATIONS.md) for live compatibility and context-control boundaries.

The TF0000 Manifest V3 extension captures visible ChatGPT, Claude, and Gemini conversations and explicit selections. The desktop workspace can merge and search whole captured chats, message ranges, fragments, existing memories, and context packs locally. Search includes structured filters, source previews, jump-to-source, and non-LLM evidence lookup. The extension also supports conversation-specific switches, temporary context lifetimes and counters, ordered context packs, presets, exact previews, manual imports, full/minimal/custom handoffs, and secret-like text warnings before saving or attaching. It never submits a prompt.

## Build

Paths using `C:\path\TF0000` are examples; replace them with your repository location.

From the repository root:

```powershell
pnpm install
pnpm build:extension
pnpm build:native-host
```

The unpacked extension is written to `apps/browser-extension/.output/chrome-mv3`. The native host executable is written to `target/release/tf0000-native-host.exe`.

## Load in Chrome or Edge

1. Open `chrome://extensions` or `edge://extensions`.
2. Enable **Developer mode**.
3. Select **Load unpacked** and choose `C:\path\TF0000\apps\browser-extension\.output\chrome-mv3`.
4. Copy the 32-character extension ID shown by the browser.
5. Register the native host for that extension ID:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\register-native-host.ps1 -ExtensionId YOUR_EXTENSION_ID -Browser Both
```

Registration is per Windows user. The script writes the native host manifest under `%LOCALAPPDATA%\TF0000\NativeMessaging` and adds only the Chrome and/or Edge native-messaging registration keys selected by `-Browser`. Re-run this command after upgrading from the former product name because the native host identifier changed.

## Manual smoke test

1. Create at least one active memory in the desktop app.
2. Open a ChatGPT, Claude, or Gemini conversation and reload the page after installing or updating the extension.
3. Select text in a message, open **TF0000**, and choose **Save selection**.
4. Try **Save messages** and **Save visible chat**. Repeating either action should update the same conversation rather than duplicate it.
5. Open a different supported provider. Select the captured source (including its badge and source link), choose **Build cross-provider preview**, inspect the exact text and token estimate, then insert it.
6. Confirm the text appears in the composer but is not sent. If insertion is unavailable, the panel reports that it copied the preview to the clipboard.
7. Choose **Run diagnostics** to check the active DOM adapter and native storage bridge. The integration screen reports the state of all three adapters.
8. On an unsupported website, open the extension and use **Manual capture / import**. Build a preview from that source; the extension uses the clipboard when direct insertion is unavailable.
9. Enable a memory or memory space under **Per-chat context**, attach an item for one or N prompts, and confirm the remaining counter changes only after insertion.
10. Build a context pack, reorder its memories, choose full/summary/reference modes, save it, and attach the resulting pack.
11. Save the current selection as a preset, clear it, then apply the preset and confirm the same targets return.
12. Open the desktop **Merge & update** workspace, select whole chats or multiple messages from different providers, add a manual note, and build the exact diff preview.
13. Apply the preview, open **History & branches**, confirm every source is shown on the new version, restore an older version, then create and merge an alternative branch.
14. Create two active decision memories with the same `Key:` and different values; confirm the deterministic conflict inbox lets you resolve or ignore the conflict.
15. Press **Ctrl+K** in the desktop app, search for captured text, and verify provider/date/type/status filters plus source previews and original-source links.
16. Switch to **Ask Memory**, try a recorded decision and an unknown phrase, and verify the results show evidence or an explicit **Not recorded** state without generating an answer.
17. In a supported conversation, choose a different provider under **Continue in…**, select full, minimal, or custom mode, and build the exact handoff preview.
18. Check the character/token estimate, then choose **Continue in**. Confirm the destination opens with the reviewed text inserted (or copied as a fallback) and that no prompt is submitted automatically.
19. Try saving or attaching text containing a test credential-like token and confirm TF0000 warns before continuing.

## Automated verification

```powershell
pnpm typecheck
pnpm test
pnpm lint
pnpm build:extension
cargo build --release -p tf0000-native-host
```

The DOM fixture tests cover provider normalization, selection anchoring, insertion, clipboard fallback, and the no-send invariant. Rust tests cover migrations, capture, nested-space cycle prevention, pack ordering and modes, bindings, temporary counters, exact composition, deterministic handoff modes, non-mutating handoff records, native framing, and authentication.
