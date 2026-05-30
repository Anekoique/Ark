# The Ark Talk

A slide deck that walks the whole stack — from what an agent *is*, up through Ark, to where it's headed. It's the narrative companion to this book: the book documents the CLI as it ships; the talk frames *why* it's shaped this way and what comes next.

The deck is a self-contained, full-screen HTML app. **Open it in its own tab for the real experience** — a preview is embedded below it. Inside the deck: arrow keys (or click) to advance, `F` for fullscreen, `O` for an overview grid.

<style>
/* Scoped to this page only. The deck is a fixed-viewport app whose fonts use
   viewport units, so it needs real width/height to render legibly — we break
   the embed out of mdBook's narrow content column to the full window. */
.ark-deck-launch{
  display:inline-flex;align-items:center;gap:.55em;
  margin:.4rem 0 1.2rem;padding:.7em 1.25em;
  font-weight:600;text-decoration:none;border-radius:8px;
  color:#0b0e14;background:linear-gradient(100deg,#5eead4,#818cf8);
  box-shadow:0 6px 20px rgba(94,234,212,.25);
}
.ark-deck-launch:hover{filter:brightness(1.05)}
.ark-deck-fullbleed{
  /* pull out of the centered, max-width content column to full window width */
  width:100vw;margin-left:calc(50% - 50vw);margin-right:calc(50% - 50vw);
  margin-top:1rem;margin-bottom:.5rem;
}
.ark-deck-fullbleed iframe{
  display:block;width:100%;height:80vh;min-height:520px;border:0;
  background:#0b0e14;
}
.ark-deck-bar{
  display:flex;justify-content:flex-end;gap:.5rem;
  max-width:1100px;margin:0 auto .5rem;padding:0 1rem;
}
.ark-deck-bar button{
  font:inherit;font-size:.85em;cursor:pointer;
  padding:.35em .8em;border-radius:6px;
  border:1px solid var(--quote-border,#444);
  background:var(--quote-bg,#1b2233);color:inherit;
}
.ark-deck-bar button:hover{border-color:#5eead4}
</style>

<p>
  <a class="ark-deck-launch" href="ark-deck.html" target="_blank" rel="noopener">
    ▶ Open the deck full-screen ↗
  </a>
</p>

<div class="ark-deck-fullbleed">
  <div class="ark-deck-bar">
    <button type="button" onclick="this.closest('.ark-deck-fullbleed').querySelector('iframe').requestFullscreen()">⛶ Fullscreen preview</button>
  </div>
  <iframe
    src="ark-deck.html"
    title="Ark — presentation"
    loading="lazy"
    allowfullscreen></iframe>
</div>

> No iframe (print, or a text reader)? Open **[ark-deck.html](./ark-deck.html)** directly.

## What's in it

Four acts, climbing the stack:

| Act | Title                   | What it covers                                                                            |
| --- | ----------------------- | ----------------------------------------------------------------------------------------- |
| ①   | Agents & harnesses      | What an agent is; framework vs. harness; the seven-dimension harness framework; why the harness is load-bearing. |
| ②   | Ark                     | Where Ark sits, the seven dimensions applied, tiers, the lifecycle, the atomic commit, subagents, specs, and the architecture. This is the part the rest of the book details. |
| ③   | ArkOS                   | RFC 001 — workflow as a service for *agents* instead of humans; the layered model; grounded self-evolution. |
| ④   | The future agent model  | A fleet of isolated sandboxes — hypervisor below, one agent-plus-project per microVM, running autonomously at scale. |

Act ② is the shipped tool; everything in [Workflow](./workflow/tiers.md) and [Reference](./reference/cli-overview.md) is its operational detail. Acts ③ and ④ are forward-looking — the direction, not yet the implementation.
