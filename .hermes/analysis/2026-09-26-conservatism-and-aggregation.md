# Is the model too conservative? The composite is a blend of two different questions

**Asked 2026-09-26.** Daim: "I still think these articles highlight critical issues with the AI
investment bubble and we are being way too conservative with our scoring and weighting of them."

This is a testable claim, so it was tested rather than debated. The short version: **the complaint
is empirically supported, and the cause is not what either of us assumed.**

---

## 1. The structural finding: the composite averages two questions whose answers differ by 28 points

Every indicator in this model answers one of four different questions. Grouped that way, today's
readings separate sharply:

| sub-question | n | weight | score |
|---|---|---|---|
| **Spending / balance-sheet strain** — what the articles describe | 10 | 78 | **53.3** |
| **Market pricing** — has it repriced *yet*? | 8 | 70 | **25.2** |
| Demand reality — is the spend justified? | 2 | 16 | 39.1 |
| Opposition — is it being resisted? | 2 | 13 | 43.2 |
| **BLENDED (what the composite reports)** | 22 | 177 | **40.2** |

A **28-point spread** between the two large blocks, and the composite reports the average.

**Why this is a defect and not a neutral choice.** The two blocks are not two views of one
quantity that should be averaged. They have different *causal roles*: the strain block is the
**precondition** for a bust; the pricing block is its **realisation**. Averaging a cause with its
own downstream effect is a category error — it guarantees that a clearly-present precondition is
reported as "middle range" for as long as the realisation has not happened yet. And the
realisation is, by construction, the *last* thing to arrive.

The mechanism the user's articles describe — the financing turning fragile — **is** the
precondition. It is reading 53.3. The composite dilutes it with a question ("has the market
noticed?") that is designed to stay calm until the answer no longer matters.

## 2. The weights themselves correlate NEGATIVELY with stress

```
n = 22      corr(weight, stress) = -0.364      t ~ -1.59
```

Weaker readings tend to carry **more** weight. The clearest case: today's two highest readings are
also two of the lowest-weighted.

| indicator | stress | weight |
|---|---|---|
| `foreign_interest` | **92.0** | 5 |
| `backlog_quality` | **89.1** | 7 |
| `credit_hy` | 18.0 | **12** |
| `valuation_stretch` | 23.1 | **14** |

**The honest caveat, stated because it cuts against the framing:** a negative correlation here is
*expected* under correct calibration. A lagging indicator — credit spreads, valuation — *should*
read low when the boom is still expanding, and carrying a large weight is defensible precisely
because it will move hard and reliably when the turn comes. So this correlation is **not by itself
evidence of mis-weighting.** It is the reason the *aggregation* is wrong, not the reason the
weights are.

## 3. Weight changes would be nearly cosmetic — the aggregation is the lever

| variant | composite | delta |
|---|---|---|
| **current (as configured)** | **40.20** | — |
| equal weights | 42.86 | +2.66 |
| weight extremes *more* (× s/50) | 43.58 | +3.39 |
| stretch the scale (× 1.5) | 57.81 | +17.61 |
| only the 4 indicators ≥ 60 | 77.52 | +37.32 |

Reweighting within the current structure moves the number by **2–3 points**. The two things that
move it meaningfully are **changing the scale** and **disaggregating the output** — neither of
which is a weight change. So "we are too conservative" is right, and "we should raise the weights"
is the wrong remedy.

## 4. What a less-conservative model should do (and the constraint on it)

**The constraint that cannot be argued away:** 12 of 22 indicators read below 40, and they include
real, measured facts — HY spreads at 2.8%, IG at 1.39%, VIX at 14.9, capex/revenue at 68%. An
article can assert that these are late and will turn; the model cannot *score* on that assertion,
because that would be scoring a forecast. The model is under a hard constraint the articles are
not: it may only score what a free source actually measures today.

**Therefore the right fixes, in order of value:**

1. **Report the sub-questions separately, not only blended.** The four-group table above should be
   in the output. It states the diagnosis — *extreme spending strain against calm pricing* — which
   is what the articles are actually pointing at, and which the single number 40 actively hides.
   This costs nothing and requires no new data.
2. **Fix the scale endpoints.** "A score above 55 would look more like the late stage of a bubble"
   is unanchored. If 55 means late-stage, that threshold needs a derivation from the two historical
   episodes the model already cites, exactly the way every other anchor in the file does.
3. **Do NOT raise weights to move the number.** It is the one lever that moves the answer by
   almost nothing, and doing it would be fudging the output to match a prior.
4. **Do NOT score the lagging indicators on their predicted future.** That is the line that turns
   this from an instrument into financial astrology, which the user explicitly does not want.

## 5. A documentation inaccuracy found on the way (and one I nearly invented)

My first pass at this flagged `config/indicators.toml` line 14 as a false claim:

> "No indicator here is inverted — including the credit spreads"

I then wrote a checker that labelled 18 indicators "INVERTED" by testing whether their anchors
*ascend*. **That checker was wrong, and so was my conclusion**, twice over:

- The header's claim is about the **stress axis** — that a higher *stress* value means *more*
  bubble-like. Under that reading it is **correct**: `credit_hy` maps a wide spread to high stress,
  and `inference_demand` maps decelerating demand to high stress. Both satisfy it.
- My checker was testing the **raw axis** (does a higher *input* map to higher stress?). That is a
  different question, and answering it wrongly made 18 correct indicators look inverted.

Recorded because the near-miss matters more than the nit: I had a "defect" written into the
analysis and was one commit from publishing it, on the strength of a checker whose own premise I
had not examined. The thing that caught it was testing the checker against a known-good case
(`credit_hy`, whose direction is documented in prose) rather than trusting the count it produced.

**The genuine, minor error is in `inference_demand`'s rationale**, which says:

> "WHY IT INVERTS: every other indicator in this model reads a higher value as more stress."

Measured: **3 other indicators** share that property — `breadth`, `frontier_premium`,
`energisation_delay`. So the claim is that it is the *only* one, and it is not. Cosmetic, but it is
a factual statement about the model's own contents, so it is being corrected to say "unlike the
majority of indicators here".


## 6. What was NOT concluded

- Not concluded that the weights are wrong — the evidence for that is the negative correlation,
  which has a legitimate innocent explanation (§2).
- Not concluded that any specific article's figures are right. The Brookings $10.3T and ~$300B
  off-balance-sheet figures remain **unverified at source** and are not in the model.
- Not concluded that 40 is "too low" in absolute terms. It is concluded that 40 **answers a
  different question** than the articles are answering, and that the difference is 28 points.
