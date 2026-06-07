# Telemetry Topic 06: Privacy, PII, Anonymization & Retention for Query Logs

## Findings
- **Stripping IDs is not enough.** The 2006 AOL release (650k users, 3 months) replaced user IDs with numbers, yet searcher #4417749 was re-identified from query *content* alone. Query text is itself a quasi-identifier.
- **GDPR distinguishes anonymized vs pseudonymized.** Only *truly* anonymized data (no reasonably-likely re-identification) escapes GDPR; hashing/pseudonyms stay in scope. IP addresses count as PII (Recital 26).
- **IR baseline = k-anonymity** plus l-diversity / t-closeness; query-log variants use microaggregation and probabilistic k-anonymity. Naive keyword hashing is breakable by statistical attack.
- **Big search engines retain then partially anonymize on a clock:** Google truncated part of the IP at 9 months, cookies at 18 months; Bing deleted full IP + cross-session IDs at 6 months. Anonymize != delete.
- **LLM-observability tools (Langfuse, LangSmith) mask client-side before egress** plus optional retention TTLs.

## What to log
- Query text and result IDs, but pass every input/output/metadata field through a **mask function before persistence** (Langfuse `mask=`, LangSmith `create_anonymizer` / `LANGSMITH_HIDE_INPUTS`).
- Detect PII with **Microsoft Presidio** (Analyzer = NER + regex + checksum recognizers; Anonymizer ops: replace/mask/redact/hash/encrypt) covering email, IP, phone, credit-card, SSN, names, dates.
- Separate identity (account/IP) from query payload at ingest; keep a separable key only if reversibility is required.
- A per-record retention timestamp/TTL and a salted-but-rotated session id (not raw IP).

## Metrics
- PII detector precision/recall and false-negative (residual-PII) rate.
- k-anonymity level / re-identification risk on quasi-identifiers.
- Retention SLA: age distribution, % records past TTL, deletion-job success.
- Masking coverage % of fields; egress-leak audit count.

## How it is used (feedback loop)
- Pipeline: Raw -> short buffer -> PII detect -> anonymize -> long-term store -> TTL cleanup. Conditional/zero-retention tracing skips sensitive requests entirely.
- Anonymized logs feed ranking, query-refinement and eval improvement while staying compliant; detector errors found in audits feed new custom recognizers.
- Tiered aging (drop IP/IDs on a clock) lets older logs be analysed at lower risk.

## Sources
- AOL / k-anonymity for query logs: https://www.sciencedirect.com/science/article/abs/pii/S0306457311000057
- Query-log anonymization survey: https://arxiv.org/pdf/1211.2354
- What to anonymize in software logs: https://arxiv.org/html/2409.11313v2
- GDPR log management pipeline: https://last9.io/blog/gdpr-log-management/
- Anonymization vs pseudonymization (GDPR): https://trustarc.com/resource/anonymization-vs-pseudonymization/
- Google retention policy: https://policies.google.com/technologies/retention?hl=en-GB
- Bing privacy update (6-month IP deletion): https://blogs.bing.com/search/January-2010/Updates-to-Bing-Privacy
- Langfuse masking: https://langfuse.com/docs/observability/features/masking
- Langfuse data retention: https://langfuse.com/docs/administration/data-retention
- LangSmith mask inputs/outputs: https://docs.langchain.com/langsmith/mask-inputs-outputs
- LangSmith PII removal demo: https://github.com/langchain-ai/langsmith-pii-removal
- Microsoft Presidio: https://github.com/microsoft/presidio
