# The Ark Talk

A slide deck that walks the whole stack — from what an agent *is*, up through Ark, to where it's headed. It's the narrative companion to this book: the book documents the CLI as it ships; the talk frames *why* it's shaped this way and what comes next.

The deck is a self-contained, full-screen HTML app. **Open it in its own tab for the real experience** — a preview is embedded below it. Inside the deck: arrow keys (or click) to advance, `F` for fullscreen, `O` for an overview grid.

<style>
/* Scoped to this page only. The deck is a fixed-viewport app: its slide layout
   (`padding:5.5vh 6vw`) and fonts (`clamp(min, Xvw, max)`) are tuned for a real
   ~1440x900 window. Sized to a narrow embed, every `vw` clamp snaps to its
   minimum and the slides render cramped and overflowing. So we render the
   iframe at a FIXED logical 1440x900 — the deck sees a full-screen viewport —
   then `transform: scale()` it down to fit the embed width. A wrapper reserves
   the scaled visual height so surrounding content reflows correctly. */
.ark-deck-launch{
  display:inline-flex;align-items:center;gap:.55em;
  margin:.4rem 0 1.2rem;padding:.7em 1.25em;
  font-weight:600;text-decoration:none;border-radius:8px;
  color:#0b0e14;background:linear-gradient(100deg,#5eead4,#818cf8);
  box-shadow:0 6px 20px rgba(94,234,212,.25);
}
.ark-deck-launch:hover{filter:brightness(1.05)}
.ark-deck-block{
  /* `--deck-embed-w` is the on-screen width of the embed: capped at 880px, but
     shrinking with the window on narrow screens. Uses `vw` (not `%`) so it
     resolves in the height/scale calc below. Both the button bar and the
     scaled frame are sized from it and centered. */
  --deck-embed-w:min(880px, 100vw - 2rem);
  margin:1rem auto .5rem;
}
.ark-deck-bar{
  display:flex;justify-content:flex-end;gap:.5rem;
  width:var(--deck-embed-w);margin:0 auto .5rem;
}
.ark-deck-bar button{
  font:inherit;font-size:.85em;cursor:pointer;
  padding:.35em .8em;border-radius:6px;
  border:1px solid var(--quote-border,#444);
  background:var(--quote-bg,#1b2233);color:inherit;
}
.ark-deck-bar button:hover{border-color:#5eead4}

/* The scaled embed. Logical size is 1440x900; `--deck-scale` is the ratio of
   the embed's display width to 1440. The frame's height is reserved as
   900 * scale so the scaled-down iframe leaves no dead space. */
.ark-deck-frame{
  --deck-w:1440px; --deck-h:900px;
  /* scale is unitless: embed width (a length) divided by the logical width
     (also a length) → a pure number, which is what `scale()` requires. */
  --deck-scale:calc(var(--deck-embed-w) / 1440px);
  position:relative;width:var(--deck-embed-w);margin:0 auto;
  overflow:hidden;border-radius:10px;background:#0b0e14;
  /* reserved visual height = display width * (900/1440) aspect ratio */
  height:calc(var(--deck-embed-w) * 0.625);
}
.ark-deck-frame iframe{
  position:absolute;top:0;left:0;
  width:var(--deck-w);height:var(--deck-h);border:0;
  transform:scale(var(--deck-scale));
  transform-origin:0 0;   /* scale from top-left so the box fills the frame */
  background:#0b0e14;
}
</style>

<p>
  <a class="ark-deck-launch" href="ark-deck.html" target="_blank" rel="noopener">
    ▶ Open the deck full-screen ↗
  </a>
</p>

<div class="ark-deck-block">
  <div class="ark-deck-bar">
    <button type="button" onclick="this.closest('.ark-deck-block').querySelector('iframe').requestFullscreen()">⛶ Fullscreen preview</button>
  </div>
  <div class="ark-deck-frame">
    <iframe
      src="ark-deck.html"
      title="Ark — presentation"
      loading="lazy"
      allowfullscreen></iframe>
  </div>
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
