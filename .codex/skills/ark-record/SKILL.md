---
name: ark-record
description: Append a manual session entry to the developer's workspace journal. Use when the user is wrapping up work that wasn't a full Ark task (research, debugging, doc edits) and wants it journaled.
---

# `ark-record`

Record a manual session entry. Identity must be initialized first via `ark agent workspace init --name <x>`.

## Preconditions

- `.ark/.state.toml`'s `[identity]` section is set (run `ark agent workspace init --name <x>` once if not).
- The conversation has produced enough work to summarize. Fresh sessions with no work yet should ask the user for a title rather than fabricate one.

## Steps

### 1. Pull session context

```bash
ark context --scope session --format json
```

Use this to anchor the summary in the right project state (branch, recent commits, current task if any).

### 2. Decide the title

If a title was provided in the user's invocation, use it verbatim. Otherwise:
- If the conversation has a clear topic, summarize the most recent topic into a 5–8 word title.
- If no work has been done yet, ask the user: *"What should I title this session?"* — do NOT fabricate.

### 3. Compose summary and next steps

From the conversation context:
- **Summary**: 1–3 sentences. Specific over generic.
- **Next steps** (optional): bullet list of follow-ups. Empty is fine.

### 4. Run the record

```bash
ark agent workspace record --title "<title>" --summary "<summary>" --next "<next steps>"
```

### 5. Report

One line summarizing the recorded session number and journal path. The CLI's own output already covers this.

## Failure modes

- `DeveloperNotInitialized` → no identity in `.ark/.state.toml`. Tell the user to run `ark agent workspace init --name <x>` first.
- `JournalRotationLimit` → unreachable in practice (>9999 journals); hand-rename old files if hit.
- `ManagedBlockCorrupt` → `<dev>/index.md` is malformed. Repair (or delete and retry).
