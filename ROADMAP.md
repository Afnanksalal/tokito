# Tokito roadmap & vision

This document states where we are going—not a promise of dates or scope order. Priorities shift as we learn from real designs.

## North star

**Altium-class depth** (rules, production workflow, serious electrical and mechanical reality)  
**plus KiCad-class freedom** (open formats, your data on disk, no mandatory cloud lock-in)  
**plus Flux-level AI** (agents that plan, research, and draft while you stay in control).

Tokito should feel like **your ideas move to a production-ready PCB** with AI working **in parallel**—not a chat box that blocks the canvas.

## Principles

- **AI alongside you** — Long jobs (research, routing passes, BOM scrubbing, DRC sweeps) run in the background; you keep editing, reviewing, and overriding.
- **You own the intent** — Suggestions are proposals; nothing ships to fabrication without explicit approval.
- **Reality over demos** — DRC, clearances, stackups, and vendor rules matter as much as pretty screenshots.
- **Interop** — Export and interchange that fit real shops (fab houses, assembly, MCAD), not a walled garden.

## Where we are today

- Native **schematic** editor: symbols, wiring, ERC, multi-sheet, exports (SVG, PDF, netlists, MCAD handoff JSON).
- **AI-assisted build**: prompt → research artifacts → BOM grounding → schematic proposal → your review before apply.
- **Sourcing-aware** part discovery (e.g. LCSC, optional Nexar-style enrichment).
- **Rough board preview** from schematic + footprint hints (not PCB layout or true 3D board modeling).

## Horizon (not exhaustive)

### Near term

- Richer **schematic** productivity: buses, harness ideas, stronger ERC, variant / assembly options, template flows.
- Tighter **library** story: footprints, 3D models, lifecycle and preferences per part.
- Clearer **handoff** to external PCB tools where Tokito does not yet replace them.

### PCB & layout

- **PCB editor**: placement, copper, planes, constraints-driven design.
- **Auto-routing** (interactive + batch): constraints-first, explainable results, pause/resume with AI.
- **DRC / DFM** aligned with real fabs; design reviews that surface “why this fails” in plain language.
- **3D** that reflects actual board geometry—not only schematic-derived previews.

### AI & automation

- **Parallel agents**: “while you route this block, optimize the power tree” / “scout alternates for this line.”
- **Design intelligence**: thermal and SI-aware nudges, testability and assembly hints, cost-aware BOM tradeoffs (with your policies).

### Production & vendors

- **Fab & assembly integrations** (e.g. JLCPCB, other LCSC-adjacent flows, and additional partners over time): quotes, uploads, BOM synchronization, acknowledging each house’s quirks in rules and checklists.
- **Shipping-aware** planning: lead time, MOQ, and revision discipline baked into the workflow—not bolted on at export.

### Platform

- Performance and stability for **large designs**; optional collaboration models without sacrificing local ownership where it matters.

---

**In one line:** from **idea → validated schematic → laid-out PCB → fab-ready pack → supported assembly path**, with **AI carrying weight while you keep the wheel**—at quality you’d expect from top-tier EDA, with the **freedom** of open stacks and the **speed** of modern AI products.

If you care about this direction and want to help shape it, open a discussion or PR; see [`CONTRIBUTING.md`](CONTRIBUTING.md).
