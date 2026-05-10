// Ark session-context plugin for OpenCode.
//
// On the user's first message of each session, shells out to
// `ark context --scope session --format json` and prepends the unwrapped
// context to the first user-text part via opencode's message-transform
// hook. Failures (missing `ark` binary, non-zero exit, JSON parse error)
// are logged via `client.app.log` and the session continues without
// injection; the first failure per session also writes a one-line stderr
// note for discoverability.
//
// Stability note: `experimental.chat.messages.transform` is the only
// opencode hook that mutates the user message before it reaches the LLM.
// If a future release renames or removes it, edit this file's hook key
// and ship via `ark upgrade`. The `chat.message` → transform ordering is
// required: the notification populates `pendingContext`, transform reads it.

import { execFileSync } from "node:child_process"

type Client = { app: { log: (args: unknown) => Promise<unknown> } }
type TextPart = { type?: string; text?: string }
type Message = { info?: { role?: string; sessionID?: string }; parts?: TextPart[] }
type TransformOutput = { messages?: Message[] }

const processedSessions = new Set<string>()
const pendingContext = new Map<string, string>()
const stderrNoticed = new Set<string>()

// Pure helpers, intentionally NOT exported. OpenCode treats every named
// export as a plugin factory and invokes it at load time with no arguments,
// which crashes any helper that takes parameters. Keep helpers internal.

function buildEnvelopePrefix(additionalContext: string): string {
  return `<ark-context>\n${additionalContext}\n</ark-context>\n\n---\n\n`
}

function shouldInject(sessionID: string, processed: Set<string>): boolean {
  return !processed.has(sessionID)
}

function warn(client: Client, sessionID: string, message: string): void {
  client.app.log({ body: { service: "ark", level: "warn", message } }).catch(() => {})
  if (stderrNoticed.has(sessionID)) return
  stderrNoticed.add(sessionID)
  process.stderr.write("ark-context: skipped context injection (see opencode logs)\n")
}

// 5s timeout caps the worst case if `ark` hangs; opencode's first-message
// path stays responsive. Throws on timeout — caller logs and continues.
const ARK_CONTEXT_TIMEOUT_MS = 5000

function fetchContext(directory: string): string {
  const raw = execFileSync("ark", ["context", "--scope", "session", "--format", "json"], {
    cwd: directory, encoding: "utf-8", stdio: ["ignore", "pipe", "pipe"],
    timeout: ARK_CONTEXT_TIMEOUT_MS,
  })
  const parsed = JSON.parse(raw) as { hookSpecificOutput?: { additionalContext?: unknown } }
  const value = parsed.hookSpecificOutput?.additionalContext
  if (typeof value !== "string" || !value) throw new Error("unexpected payload shape")
  return value
}

export default async ({ directory, client }: { directory: string; client: Client }) => ({
  "chat.message": async (input: { sessionID?: string }) => {
    const sessionID = input?.sessionID
    if (!sessionID || !shouldInject(sessionID, processedSessions)) return
    processedSessions.add(sessionID)
    try {
      pendingContext.set(sessionID, fetchContext(directory))
    } catch (err) {
      warn(client, sessionID, `ark context failed: ${(err as Error)?.message ?? String(err)}`)
    }
  },

  "experimental.chat.messages.transform": async (_input: unknown, output: TransformOutput) => {
    const lastUser = output.messages?.findLast((m) => m.info?.role === "user")
    const sessionID = lastUser?.info?.sessionID
    if (!sessionID) return
    const additionalContext = pendingContext.get(sessionID)
    if (!additionalContext) return
    const textPart = lastUser.parts?.find((p) => p.type === "text" && typeof p.text === "string")
    if (!textPart) return // no text part on this turn — keep context for the next one
    textPart.text = buildEnvelopePrefix(additionalContext) + (textPart.text ?? "")
    pendingContext.delete(sessionID)
  },
})
