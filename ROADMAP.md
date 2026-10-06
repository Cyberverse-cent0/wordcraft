# WordCraft roadmap

WordCraft aims for complete feature parity with Microsoft Word, then goes further on speed, openness
and agent control. This file tracks where we are honestly. Generated numbers come from
`cargo xtask parity` (`docs/parity.md`).

## Where we are (2026-10-07)

| Measure | Value |
|---|---|
| Commands (every action, scriptable by CLI/MCP/control channel) | **389** |
| Feature catalog coverage (Word ribbon/menu features with a command) | **354 / 405 (87%)** |
| **Estimated real feature parity** (depth and fidelity, not just a command) | **~58%** |
| Tests | ~240 (unit, round-trip, fuzz/proptest, MCP acceptance) |
| Layout speed (188-page document) | 61 ms cold, **1.4 ms** relayout after an edit |
| Code | ~42k lines of Rust in 15 crates |

Catalog coverage overstates parity: many commands are first versions. The estimate weighs each
area by how much of Word's behaviour it reproduces.

| Area | Status | Parity |
|---|---|---|
| Typing, selection, clipboard, undo, find/replace | Solid; IME, autocorrect, smart quotes, list autoformat | 85% |
| Character & paragraph formatting | Nearly all properties; Font/Paragraph dialogs | 85% |
| Styles (gallery, pane, create/modify, style sets, themes) | Good | 75% |
| Lists (bullets, numbering, multilevel, restart, set value) | Good | 75% |
| Tables (insert, merge/split, styles, borders, header rows, sort, formula) | Good; no row splitting across pages, no drawn tables | 65% |
| Page layout (margins, size, orientation, columns, breaks, sections) | Good; columns don't balance, line numbers/page borders/vertical alignment not drawn | 60% |
| Headers/footers, page numbers, fields | Good; first/even/odd, link to previous | 70% |
| Footnotes/endnotes | Placed and editable; long notes don't continue onto the next page | 60% |
| References (TOC, citations APA/MLA/Chicago/IEEE, bibliography, index, figures, cross-refs, TOA) | Working first versions | 60% |
| Review (spelling, grammar, thesaurus, comments, track changes, compare, protect) | Good; comments show in a pane, not margin balloons | 65% |
| Mailings (mail merge, rules, preview, envelopes, labels) | Working | 70% |
| Pictures & shapes (insert, size, crop, recolour, effects, styles, float position) | Text does not yet wrap around floating objects; text boxes don't render their text | 40% |
| Draw tab (ink), SmartArt, charts, 3D models, equations editor | Not started / linear equations only | 5% |
| File formats: DOCX read/write | Good (Word opens our files); charts/SmartArt/OLE dropped | 75% |
| File formats: PDF, ODT, RTF, HTML, Markdown, TXT | Working | 70% |
| View modes (print, web, draft, read, focus, zoom, navigation pane) | Working | 70% |
| Backstage (new from templates, open, info, export, options) | Working; printing goes through PDF | 55% |
| Agent control (CLI, MCP, control channel, macros) | Beyond Word | 100%+ |

## Estimate to 100%

**About 140–180 hours of Claude Opus 5.5 wall-clock work** (with parallel agents), in this order:

1. **Text wrap around floating objects, text box rendering, drop caps** (≈15 h) — biggest visual gap.
2. **Layout fidelity**: row splitting across pages, column balancing, line numbers, page borders,
   vertical alignment, footnote continuation, auto-hyphenation, kerning/ligature options (≈25 h).
3. **Comments in margin balloons, track-changes balloons, formatting revisions** (≈12 h).
4. **Draw tab / ink, shapes on a canvas, grouping, rotation, z-order** (≈20 h).
5. **Charts (own renderer) and SmartArt-style diagrams** (≈20 h).
6. **Equation editor (OMML read/write, 2D layout)** (≈15 h).
7. **Dialog depth**: every Word dialog with all its options (Font, Paragraph, Tabs, Borders and
   Shading, Page Setup, Styles, Columns, Index/TOC options, Mail Merge wizard, Options panes) (≈20 h).
8. **DOCX fidelity corpus**: round-trip and render a public corpus against Word, fix differences (≈20 h).
9. **Native printing, accessibility (screen readers), localisation, RTL/complex scripts** (≈15 h).
10. **Packaging and signed releases for every platform** (≈5 h, pipeline copied from PhotoCraft).

## Milestones

| # | Milestone | State |
|---|---|---|
| M0 | Skeleton + vertical slice (model, layout, render, engine, Word-style UI, CLI, MCP, web) | **done** |
| M1 | DOCX I/O | **done** (first version) |
| M2 | Home tab complete | **done** |
| M3 | Insert tab | mostly done (charts/SmartArt/icons/3D missing) |
| M4 | Layout + Design tabs | mostly done (rendering gaps above) |
| M5 | Tables | mostly done |
| M6 | References | first version done |
| M7 | Review | mostly done |
| M8 | View | mostly done |
| M9 | Mailings | done (first version) |
| M10 | File/Backstage | mostly done |
| M11 | Draw + objects | started |
| M12 | Formats breadth (PDF, ODT, RTF, HTML, MD, TXT) | **done** (first versions) |
| M13 | Performance budgets | on track (1.4 ms relayout) |
| M14 | 1.0 polish, packaging, signing | not started |

## Current focus
Floating-object text wrap and text boxes; layout fidelity items; README screenshots; release pipeline.
