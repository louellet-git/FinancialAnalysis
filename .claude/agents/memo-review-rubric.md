# Consulting-Grade Memo & Word-Document Review Rubric

Canonical reference for the `memo-reviewer` agent. Grades memos / Word documents against
client-partner standards (MBB / Deloitte deliverables + the Amazon narrative-memo tradition).
Sibling to `deck-review-rubric.md`: use the **deck** rubric for `.pptx`, **this** for `.docx` / prose.

A memo is prose that must stand alone and be read in silence — so the weight shifts toward
writing quality and, because the artifact is a Word document, toward typography.

## 1. Two axes
- **Axis A — Substance (thinking & writing):** one clear, answer-first argument, clean prose, sourced evidence, pitched at the right reader.
- **Axis B — Craft (the document):** scannable, typographically professional, consistent, error-free.
Substance ~55-60%, craft ~40-45% (more craft than the deck rubric: a doc has no presenter to cover for ugly type or a wall of text).

| # | Category | Axis | Weight |
|---|----------|------|--------|
| 1 | Structure & Argument Logic | Substance | 18% |
| 2 | Writing Quality & Clarity | Substance | 22% |
| 3 | Tone & Audience Fit | Substance | 9% |
| 4 | Evidence, Data & Sourcing | Substance | 11% |
| 5 | Scannability & Navigation | Craft | 11% |
| 6 | Typography & Readability | Craft | 15% |
| 7 | Formatting Consistency & Polish | Craft | 14% |

## 2. Two-pass method
- **Pass 1 — Document level (skim test).** Read, in order: subject line, exec summary / opening para, every heading, and the first sentence of every paragraph. Does the **answer come first (BLUF)**? Do headings + topic sentences alone convey the argument? Is it MECE? Sweep global craft (font, size, spacing, margins, styles, page numbers, length). Scores cat 1, 5, 6, 7 + doc-level of 2.
- **Pass 2 — Section/paragraph level.** Score writing (2), tone (3), evidence (4) per section; collect red flags; note the one key fix.
- **Aggregate:** weighted score, red-flag caps, tier, prioritized fixes.

### Assessment modes (officecli pipeline)
- **text** — `officecli view <doc> text`: structure, writing, tone, in-text sourcing. Compute readability metrics from it.
- **structure** — `officecli get <doc> / --json`, `query`: font family/size, line spacing, paragraph/heading styles, page numbers, straight-vs-curly quotes, double-spaces, "--" dashes, list usage. (`officecli view <doc> issues` too.)
- **vision** — `officecli view <doc> screenshot --page N -o /tmp/x.png` then Read: white-space balance, margins, crammed-vs-airy. If an input is unavailable, mark the criterion `not_assessed` — do not hallucinate a typographic grade.

## 3. Scale (1-4)
1 Bad (embarrass at a client) · 2 Passable (amateur, needs rework) · 3 Good (manager-ready) · 4 Excellent (partner-ready, send to a board).

## 4. Categories (anchors)
1. **Structure & Argument Logic** (text, 18%) — BLUF (answer first), Pyramid/MECE, SCQA opening, one-page exec summary for >2pp. Excellent: answer + "so what" in the first paragraph; argument complete from summary + headings alone.
2. **Writing Quality & Clarity** (text, 22%, highest) — Amazon narrative bar: topic sentences, paragraphs that lead with the point and connect causally; concise, active voice, plain words, jargon explained, no filler. Excellent: tight confident narrative, nothing to cut. Example: "It was determined that a reduction in churn could potentially be achieved" -> "We can cut churn 6 points by fixing onboarding."
3. **Tone & Audience Fit** (text, 9%) — register right for THIS reader; confident not hedged/academic/chatty; framed around their decision; anticipates and answers their questions; jargon calibrated (define on first use).
4. **Evidence, Data & Sourcing** (text/structure, 11%) — every claim supported, **every number sourced** and specific ("churn rose 8%->14%", not "churn increased"); exhibits captioned and referenced; detail in labeled appendices.
5. **Scannability & Navigation** (vision/structure/text, 11%) — specific parallel headings that outline the doc; short paragraphs, white space; lists used judiciously (a NARRATIVE memo deliberately minimizes bullets — judge against intended style); TOC + page numbers for length.
6. **Typography & Readability** (structure/vision, 15%, the aesthetic core; Butterick) — body 10-12pt; line spacing 120-145%; line length 45-90 chars; a deliberately chosen professional font (avoid Times New Roman / Arial defaults and goofy/monospaced); <=2 typefaces; ONE space after a period; curly quotes; correct en/em dashes (not "--"); bold/italic sparingly, never together; first-line indent OR space between paragraphs, not both.
7. **Formatting Consistency & Polish** (structure/text/vision, 14%) — style-driven (Word styles, not manual one-offs), clean heading hierarchy, consistent margins/spacing, minimal table rules, consistent capitalization/terminology, zero typos/placeholders/broken refs, correct client name throughout.

## 5. Red-flag auto-caps
**Standard (cap at Passable ≤2):** no BLUF (answer not in opening/exec summary); any number without a source; a multi-page memo with no headings (wall of text); leftover placeholder text or broken cross-references.
**Severe (hard fail -> Bad):** data misrepresented / claims unsupported by the cited evidence / fabricated-looking stats; recommendation contradicts the body; confidentiality breach (wrong client name/data, unredacted sensitive info).

## 6. Score, tiers, output
**Weighted** = Σ(weight × score) on 1-4; `score_0_100 = (weighted-1)/3*100`. Report `substance_score` and `craft_score` separately.

| Weighted | Tier | Verdict |
|---|---|---|
| ≥ 3.50 | **Partner-ready** | Send it. |
| 2.80-3.49 | **Manager-ready** | Good; targeted edits. |
| 2.00-2.79 | **Needs work** | Not client-grade. |
| < 2.00 | **Rebuild** | Rework from the argument up. |

Then apply §5 caps. Emit JSON: `doc_type`, `intended_style` ("structured"|"narrative"), `overall{weighted_score,score_0_100,tier,tier_capped_by_red_flag,substance_score,craft_score,confidence,needs_human_review}`, `categories[]{id,name,score,weight,assessed,evidence,top_fix}`, `readability_metrics{avg_sentence_length,passive_voice_pct,avg_paragraph_words,reading_grade,undefined_acronyms[],topic_sentence_rate}`, `red_flags[]{severity,type,section,detail}`, `skim_test{exec_summary_present,answer_in_opening,heading_outline[],topic_sentences[]}`, `prioritized_fixes[]`, `per_section[]`.

## 7. Level-up practices
Be modality-aware (text + .docx structure + render). Emit the **skim-test** artifact (exec summary + heading outline + first sentence of every paragraph). Ground writing scores in **real metrics** (avg sentence length, passive %, paragraph length, reading grade, acronym count, topic-sentence rate). Don't just grade — give before->after for the buried bottom line (surfaced as a one-sentence BLUF) and the weakest paragraph. Report confidence; escalate borderline/low-confidence to `needs_human_review`. **Decide narrative vs structured up front** and flex Category 5 (don't penalize a narrative memo for no bullets, or a structured one for using them well). Take CONTEXT (audience, purpose, decision). Memo-type presets: decision memo, findings/recommendation report, info/update, PR-FAQ.

## 8. Grounding
Minto (Pyramid, MECE, SCQA); BLUF doctrine; the Amazon narrative-memo / working-backwards PR-FAQ tradition; Butterick *Practical Typography* (10-12pt, 120-145% spacing, 45-90 char lines, professional fonts, curly quotes, one space, correct dashes, minimal table rules); business-memo guidance (parallel headings, short paragraphs, exec summaries, jargon control, anticipate the reader's questions).
