# Telemetry Topic 35: Drift Detection (embedding drift, data drift, query-distribution shift)

## Findings
Practitioners treat retrieval/embedding drift as a *leading indicator*: the API still returns 200 OK and top-k chunks, but semantic alignment between queries and corpus silently erodes before answer quality drops. Three distinct drift sources are monitored separately: (1) corpus/reference-data drift, (2) query-distribution / intent shift, and (3) coverage mismatch (how well the corpus covers incoming queries). Arize's GA approach for unstructured data uses **Euclidean distance between the production-set centroid and a baseline centroid** (cosine distance, hyperbox-IOU, and clustering group-purity are alternatives), because structured-data metrics (PSI, KL, JS) don't directly extend to high-dimensional embeddings. UMAP/t-SNE plus **HDBSCAN clustering** (in open-source Phoenix) is used for root-cause: spotting new query clusters with low retrieval precision. For dense retrieval, academics propose a deployment-time **query shift score (QSS)** = divergence of a query embedding from the centroid of training-query embeddings. Search-engine work detects *intent shift* from session logs (queries + clicks), motivated by ~50% of traffic being news/seasonal/pop-culture driven.

## What to log
- Per-query: query embedding (or its centroid contribution), query length/structure, timestamp.
- Per-retrieval: top-k cosine similarities (query↔retrieved docs), "absence-of-results" flag.
- Corpus: document embeddings, ingestion/embed errors, staleness/version of index.
- Reference baseline window (frozen embeddings + golden probe set of 50–100 query-context pairs).
- Outcome signals: clicks, LLM-judge scores on a sampled subset.

## Metrics
- Euclidean distance between production vs baseline embedding centroids; cosine distance.
- KS 2-sample test layered on the distance samples for alerting rigor.
- PSI / KL / JS divergence on reducible feature distributions; QSS for query drift.
- Rolling mean & std-dev of query↔doc similarity; alert at sustained drop > 2σ.
- Recall@K / retrieval precision against the golden probe set; absence-of-results rate.
- Cluster-count / cluster-purity (HDBSCAN), OOD-query rate (one-class SVM / autoencoder reconstruction error).

## How it is used
Drift detection (centroid distance + KS) raises an alert; UMAP+HDBSCAN drill-down identifies the drifting/low-precision cluster, which prioritizes what to label next and focuses re-embedding/retraining. Self-healing pipelines re-embed changed docs, re-score against golden questions (Recall@K, answer variance, cosine drift), then promote or auto-rollback the embedding update. New low-precision query clusters flag corpus-coverage gaps (content to add).

## Sources
- Arize, Monitoring Embedding/Vector Drift Using Euclidean Distance — https://arize.com/blog-course/embedding-drift-euclidean-distance/
- Arize AX Docs, Embedding Drift — https://arize.com/docs/ax/machine-learning/computer-vision/how-to-cv/embedding-drift
- Phoenix, Embeddings Analysis (HDBSCAN, query distance) — https://arize.com/docs/phoenix/cookbook/retrieval-and-inferences/embeddings-analysis
- APXML, Monitoring Drift in Retrieval Components (RAG) — https://apxml.com/courses/optimizing-rag-for-production/chapter-6-advanced-rag-evaluation-monitoring/monitoring-retrieval-drift-rag
- AWS, Monitor embedding drift for LLMs (SageMaker JumpStart) — https://aws.amazon.com/blogs/machine-learning/monitor-embedding-drift-for-llms-deployed-from-amazon-sagemaker-jumpstart/
- Adapting to the Shifting Intent of Search Queries (arXiv 1007.3799) — https://arxiv.org/pdf/1007.3799
- Intent Shift Detection Using Search Query Logs — https://aclanthology.org/O11-4004.pdf
- Langfuse Observability Overview (trace logging incl. retrieval/embedding) — https://langfuse.com/docs/observability/overview
