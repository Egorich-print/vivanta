# ADR-001: RFC Freeze

Status: Accepted

Context: The project has accumulated 10 RFCs over three milestone phases (M1, M2, M3). Continued RFC creation without hardware validation risks speculative architecture. Grok and DeepSeek reviews both identified this as a risk.

Decision:
- RFC-001 through RFC-009 are **Accepted** — validated by experiment or architecture review.
- RFC-010 (Memory Resource Model) is **Experimental** — requires hardware validation before acceptance.
- New RFCs are **frozen** until M1-B0 (UART output on lavender) completes.
- After M1-B0, RFC creation resumes with a new rule: every RFC must include a minimal experiment.

Consequences: Speculative architecture is capped. Hardware bringup defines the next specification cycle. Existing RFCs remain authoritative.
