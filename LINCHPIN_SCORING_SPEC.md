# LINCHPIN Scoring and Red-Team Specification

LINCHPIN does not calculate "probability of getting a patent." It produces independent scored dimensions plus evidence coverage and uncertainty.

## Dimension vector (0-100 where score is supported by calibrated evidence)
- `market_pull`: severity/frequency/budgeted demand.
- `need_durability`: structural persistence over time, not trend heat.
- `white_space`: scarcity of close substitutes/solutions after search.
- `novelty_resilience`: resistance to single-reference anticipation attacks.
- `obviousness_resilience`: resistance to plausible reference combinations/motivations.
- `support_depth`: fallback embodiment/definition/figure/claim support.
- `design_around_resistance`: difficulty retaining outcome while avoiding limitations.
- `standards_chokepoint_leverage`: likelihood implementation intersects required interoperability/standard/physical constraint; never claims SEP status without evidence.
- `licensing_leverage`: number/quality of identifiable implementers/buyers and observable value capture.
- `feasibility`: technical/time/capital practicality.

## Penalties kept separate
FTO warning density, regulatory burden, capital intensity, implementation secrecy risk, prior-public-disclosure risk, inventor-contribution ambiguity, dependency on unverified evidence, concentrated buyer power, rapidly shifting technical baseline.

## Evidence coverage
Each dimension records `score`, `confidence`, `evidence_ids`, `contrary_evidence_ids`, `assumptions`, `freshness`, and `model_disagreement`. Missing evidence lowers confidence; it never becomes a neutral 50.

## Red-team sequence
1. novelty killer search;
2. obviousness combination builder;
3. prior-use/product/manual/NPL search;
4. independent Design-Around Tournament;
5. enablement/support attack;
6. market substitute and "do nothing" alternative attack;
7. commercialization buyer-budget attack;
8. disclosure/inventorship sanity gate.

A candidate can be promoted to `ARCHITECTURE_READY` only when all eight have evidence-backed dispositions and the user has confirmed the human-conceived inventive mechanism to be drafted.
