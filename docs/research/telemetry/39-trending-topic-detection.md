# Telemetry #39: Trending / Emerging Topic Detection from Query Streams

## Findings
Production trend detection is built on **velocity vs. a baseline**, not raw volume. Kleinberg's burst model (infinite-state automaton, hierarchical bursts) is the canonical IR foundation; Twitter/X-style systems mark a term trending when its recent frequency exceeds its expected frequency by a large margin (z-score on a binomial model). Search engines (Google Trends "rising/breakout") and e-commerce pipelines aggregate queries over hours-to-days, score burstiness, then **merge semantically-related queries** (shared terms / shared result sets) into coherent topics, often enriched with geography. RAG/observability tools (LangSmith Insights, Arize Phoenix, Nomic Atlas) instead **embed queries and cluster** them, auto-label clusters with an LLM, and track cluster size/novelty over time to surface emerging behaviour. KB-search practice treats **zero-result and rising queries** as a demand-forecasting / content-gap signal. The hard part everywhere is separating signal from noise: low-baseline spikes (+2000% on 10 searches), statistical noise at low volume, and the self-reinforcing feedback once a topic is surfaced.

## What to log
- Per-query: normalized query text, embedding vector, timestamp, (geo / user-segment).
- Rolling per-term / per-cluster frequency counts in short windows (hours/days) vs. a long-term baseline.
- Zero-result and low-confidence (no good hit) flags per query.
- Cluster assignment + auto-label, cluster first-seen time, cluster size history.
- Result set / top doc IDs (to enable shared-result merging).

## Metrics
- **Burst / z-score**: (observed - expected) / sqrt(n·p·(1-p)); momentum via EMA/MACD.
- **Rising %** vs. previous period (with absolute-volume floor to kill low-baseline noise).
- **Novelty**: cluster age + size growth rate; new-cluster detection.
- **Zero-result rate**, search success rate (target 80%+), CTR, deflection rate.
- Geographic/segment entropy to localize vs. global trends.

## How it is used (feedback loop)
- Threshold-cross alerts (e.g. >=50 occurrences / 7 days) auto-add topics to a content-gap queue.
- Distinguish **content gap** (no doc exists -> ingest/write it) from **vocabulary gap** (doc exists, terms mismatch -> add synonyms/aliases, improve retrieval).
- Prioritize by volume x impact x urgency; weekly "top-N no-result" cadence feeds the ingestion pipeline.
- Surfaced trends feed recommendations/autocomplete (one system saw +6% engagement from local topics); cluster insights drive eval/prompt fixes.
- Guard against the trending-feedback confounder (surfacing inflates the measured signal).

## Sources
- Kleinberg, Bursty and Hierarchical Structure in Streams: https://www.researchgate.net/publication/2842034_Bursty_and_Hierarchical_Structure_in_Streams
- BurstSketch (sketch-based burst detection): https://yangtonghome.github.io/uploads/burst_detection.pdf
- Detecting spiking queries (patent): https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/8150841
- Real-Time Classification of Twitter Trends (z-score / baseline): https://arxiv.org/pdf/1403.1451
- Latent Source Model for Time Series (spike normalization): https://arxiv.org/pdf/1302.3639
- Google Trends: reading rising queries / low-baseline pitfalls: https://www.bossistatistik.com/en/blog/how-to-read-google-trends-data-search-interest-rising-queries.html
- Google Trends FAQ (noise at low volume): https://support.google.com/trends/answer/4365533?hl=en
- LangSmith observability (topic clustering / Insights): https://www.langchain.com/langsmith/observability
- Nomic Atlas topic modeling (embed -> cluster -> LLM-label, hierarchy): https://docs.nomic.ai/atlas/capabilities/topics
- Mining search logs for content gaps (zero-result / rising): https://www.earley.com/insights/mining-search-logs-content-strategy
- Documentation gaps via analytics + MCP threshold monitoring: https://document360.com/blog/documentation-gaps/
