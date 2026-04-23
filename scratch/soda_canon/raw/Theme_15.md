# 🧠 SODA CANONICAL KNOWLEDGE BASE
> Gerado pelo @soda-knowledge-curator.
> Fontes Originais: 249 | Fontes Puras: 247 | Temas Identificados: 24



## 🧩 Eixo Temático 15

# HISA: Efficient Hierarchical Indexing for Fine-Grained Sparse Attention
Source URL: https://arxiv.org/html/2603.28458v3

Source Type: web_page

Source ID: 95a57c45-4c37-4d88-b549-6e61acfc7741


HISA: Efficient Hierarchical Indexing
for Fine-Grained Sparse Attention
Abstract
Token-level sparse attention mechanisms, exemplified by DeepSeek Sparse Attention (DSA), achieve fine-grained key selection by scoring every historical key for each query through a lightweight indexer, then computing attention only on the selected subset. While the downstream sparse attention itself scales favorably, the indexer must still scan the entire prefix for every query, introducing an per-layer bottleneck that grows prohibitively with context length. We propose HISA (Hierarchical Indexed Sparse Attention), a plug-and-play replacement for the indexer that rewrites the search path from a flat token scan into a two-stage hierarchical procedure: (1) a block-level coarse filtering stage that scores pooled block representations to discard irrelevant regions, followed by (2) a token-level refinement stage that applies the original indexer exclusively within the retained candidate blocks. HISA preserves the identical token-level top- sparse pattern consumed by the downstream Sparse MLA operator and requires no additional training. On kernel-level benchmarks, HISA achieves up to speedup at 64K context. On Needle-in-a-Haystack and LongBench, we directly replace the indexer in DeepSeek-V3.2 and GLM-5 with our HISA indexer, without any finetuning. HISA closely matches the original DSA in quality, while substantially outperforming block-sparse baselines.
1 Introduction
Serving large language models (LLMs) (OpenAI, 2026; Anthropic, 2026; Google DeepMind, 2025; Meta, 2025; Qwen, 2026; DeepSeek-AI, 2024; MiniMax et al., 2025; Moonshot AI, 2025) over long contexts remains a central systems challenge. As context windows grow from 128K to 1M tokens and beyond—driven by demands for agentic multi-turn reasoning, long-document understanding, and native multimodal processing—the quadratic cost of self-attention becomes a dominant bottleneck in both prefill latency and memory consumption (Dao et al., 2022; Dao, 2023).
A productive line of work tackles this challenge through sparse attention: instead of attending to all key–value pairs, each query selects a small subset of the most relevant tokens and computes attention only over that subset. DeepSeek-V3.2 (DeepSeek-AI, 2025) adopts a token-level sparse attention paradigm, in which a lightweight indexer scores every historical token for each query, selects the top- highest-scoring keys, and forwards only those keys to a downstream Sparse Multi-Head Latent Attention (Sparse MLA). This design has also been adopted in GLM-5 (GLM-5-Team, 2026) and provides strictly finer-grained selection than block-level methods such as MoBA (Lu et al., 2025) and Native Sparse Attention (Yuan et al., 2025).
However, the token-level sparse paradigm introduces a subtler bottleneck. Although the downstream attention is sparse and cheap, the indexer itself must score every token in the prefix for every query. Concretely, if the prefix length is and the indexer runs once per query per layer, the per-layer indexing cost is —the same asymptotic scaling as dense attention. As context lengths push toward 128K or 1M tokens, the indexer can transition from a negligible overhead into the dominant cost component.
This observation motivates a natural question: can we reduce the indexer’s search cost without changing the final sparse attention pattern it produces? In other words, can we rewrite the search path while preserving the search result?
We answer affirmatively with HISA (Hierarchical Indexed Sparse Attention). HISA replaces the flat, full-prefix token scan with a two-stage hierarchical search (shown in Figure 1):
-
1.
Block-level coarse filtering. The prefix is partitioned into contiguous blocks of size . A pooled representative vector is computed for each block via mean pooling over its constituent indexing keys. The query scores all block representatives and retains only the top- blocks, immediately pruning the majority of the prefix from further consideration.
-
2.
Token-level refinement. The token-level indexer then scores at most tokens from the candidate blocks using the same scoring mechanism as the original DSA indexer, except that the candidate pool is restricted to the tokens within the selected blocks rather than the full set of tokens considered in DSA. The final top- token set is then selected from this reduced candidate pool.
Crucially, HISA produces outputs with the same structure as the original DSA indexer: for each query, a set of token indices. As a result, the downstream Sparse MLA operator remains entirely unchanged. HISA is therefore a drop-in replacement that requires no retraining, no architectural changes to the attention mechanism, and no modification to the KV cache layout. The per-query indexing complexity drops from to , and the per-layer cost drops from to .
Our contributions are as follows:
-
•
We identify the indexer as an emerging bottleneck in token-level sparse attention systems and formalize the problem of search-path optimization for sparse indexers.
-
•
We propose HISA, a hierarchical block-to-token indexing strategy that is training-free, operator-compatible, and asymptotically faster than the flat indexer.
-
•
We provide optimized TileLang GPU kernel implementations for both stages of HISA and demonstrate – kernel-level speedup at 64K contexts.
-
•
We empirically validate that HISA achieves performance comparable to the original DSA on the Needle-in-a-Haystack and LongBench benchmarks.
2 Related Work
Block sparse attention.
Block sparse attention partitions sequences into fixed-size blocks and restricts computation to selected blocks, mapping naturally to GPU tiled matrix multiplications. This design is hardware-friendly, but all tokens within a block must be retained or discarded together. Among training-free methods, MInference (Huiqiang et al., 2024) profiles each head offline and assigns one of several sparse patterns at inference time; FlexPrefill (Lai et al., 2025) estimates block scores online and selects blocks by a cumulative-attention threshold; XAttention (Xu et al., 2025) uses antidiagonal sums as an proxy for block importance; and SpargeAttention (Zhang et al., 2025) applies a two-stage online filter to skip low-importance regions during matrix multiplication and softmax. Among trainable methods, MoBA (Lu et al., 2025) uses mixture-of-experts-style routing over blocks, while NSA (Yuan et al., 2025) combines compression, selection, and sliding-window branches to cover different dependency scales. Their common limitation is block granularity: they cannot capture token-level importance differences within a selected block. HISA also introduces a block-level stage, but only as a fast pre-filter before token refinement; its final sparse pattern remains fine-grained and token-wise, as in DSA.
Token sparse attention.
Token-level methods offer finer selection but face the challenge of efficient importance estimation. SnapKV (Yuhong et al., 2024) uses an observation window at the end of the prompt to select important KV positions for subsequent decoding, but ignores layer- and query-specific variation. KV cache eviction methods—such as H2O (Zhang et al., 2024), which combines cumulative attention with recency, and TOVA (Oren et al., 2024), which evicts the lowest-scoring cached token under the latest query—maintain a fixed-size cache but irrecoverably lose evicted tokens. LazyLLM (Fu et al., 2024) progressively prunes tokens across layers during prefill, so early pruning mistakes cannot be corrected later in the same forward pass. DSA (DeepSeek-AI, 2025) instead scores every prefix token with a lightweight indexer and selects top- tokens per query, achieving fine-grained sparsity at the cost of per-layer indexing overhead. IndexCache (Bai et al., 2026) reduces this cost by reusing indices across nearby layers, although its benefit depends on cross-layer similarity in sparse patterns.
Hierarchical sparse attention.
Hierarchical attention dates back to Yang et al. (2016), who introduced a two-tier word-and-sentence network for document classification. Among recent sparse methods, NSA (Yuan et al., 2025) and InfLLM-V2 (Zhao et al., 2026) can both be viewed as two-level designs: they score block-level summaries globally and activate finer-grained sparse attention only within selected blocks. Twilight (Lin et al., 2025) uses quantized keys for coarse scoring and then applies hierarchical top- pruning, while Double-P (Ni et al., 2026) clusters the KV cache, scores cluster centroids, refines computation within selected clusters, and approximates low-score clusters with their centroids. HISA follows the same coarse-to-fine spirit but with a different goal: it combines a hardware-friendly block-level indexer with a fine-grained token-level indexer to accelerate DSA, achieving both high efficiency and strong selection quality on DeepSeek-V3.2 and GLM-5.
3 Preliminary
We briefly review DeepSeek Sparse Attention (DSA) as used in DeepSeek-V3.2 (DeepSeek-AI, 2025). DSA consists of two components: a token-wise Indexer and Sparse MLA.
Indexer in DSA.
Let denote the causal prefix length for a query position . The indexer maintains lightweight indexing keys , indexing queries for indexing heads, and per-head gating weights . The relevance score between query and key is defined as
| (1) |
The indexer then selects the top- token indices,
| (2) |
which are passed to the downstream Sparse MLA operator. Since the scoring cost for each query is over the full prefix, the total cost across all queries in a layer is .
Sparse MLA in DSA.
Following the DeepSeek-V3.2 design, Sparse MLA adopts the MQA mode of MLA, in which each token stores a single latent key–value entry shared across all query heads for efficiency. Let denote the latent MLA entry associated with token . Given the selected token set , Sparse MLA computes attention for query token only over the selected latent entries, rather than over the full prefix:
| (3) |
As a result, the main attention cost is reduced from dense to sparse . For our purposes, the key observation is that the interface between the two components is precisely the selected token set : HISA replaces only the indexer search path, while leaving the downstream Sparse MLA operator unchanged.
4 Method
4.1 HISA: Hierarchical Indexed Sparse Attention
As shown in Figure 1, HISA replaces the flat prefix scan with a two-stage coarse-to-fine search. The final output remains an identical per-query token set of size , consumed by the original Sparse MLA operator.
Block partitioning and pooled keys.
The prefix tokens of length is partitioned into contiguous, causally valid blocks , where is the block size. For each block, a representative key is constructed via mean pooling over its indexing keys:
| (4) |
These representative keys serve exclusively as coarse-grained proxies for block-level scoring and leave both the token-level indexing keys consumed by the second stage and the KV states consumed by Sparse MLA unchanged, thereby making HISA a plug-and-play replacement. In practice, these representative keys can be incrementally maintained alongside the KV cache with negligible overhead.
Stage 1: Block-level coarse filtering.
For query position , HISA reuses the same indexing query representations and gating weights as DSA, but scores the pooled representative keys instead of individual token keys:
| (5) |
The top- blocks are selected:
| (6) |
and the candidate token set is the union of all tokens in the selected blocks:
| (7) |
All block selections strictly respect the causal mask: only blocks that precede the query position , together with the block containing position , are considered eligible. Following MoBA (Lu et al., 2025), the first and the last blocks are always included in , as they contain the attention sink and local contexts. This forced inclusion also simplifies boundary handling during batched prefill with packed sequences of varying lengths, where a single block may straddle the boundary between two sequences.
Stage 2: Token-level refinement.
Within the selected candidate set , the token-level indexer computes scores using the same scoring mechanism as in the original DSA (Eq. 1):
| (8) |
Then the top- tokens are selected as final tokens:
| (9) |
To ensure that the candidate pool is sufficiently large to select tokens, the feasibility constraint must be satisfied. Given the selected token set , sparse MLA is executed following the same computation as in the original DSA. Algorithm 1 provides the complete pseudocode for the HISA indexer.
Boundary behavior.
Three regimes arise depending on the relationship between the effective prefix length , the candidate capacity , and the budget :
-
•
When , all prefix tokens are selected and HISA is equivalent to dense attention.
-
•
When , the coarse filter selects all blocks (since ), and Stage 2 reduces the set to tokens. HISA is equivalent to the original DSA indexer.
-
•
When , the coarse filter performs non-trivial block pruning, activating HISA’s hierarchical advantage, which becomes increasingly pronounced as the sequence length grows.
The third regime is precisely the long-context setting where HISA provides its efficiency gains.
4.2 Complexity Analysis
Assuming that the pooled representative keys are maintained incrementally, the per-query indexing cost of HISA consists of scoring block representatives (Stage 1) and scoring at most candidate tokens (Stage 2):
| (10) |
Summing over all queries within a layer yields:
| (11) |
compared to for the original DSA indexer. The design introduces a clear trade-off: larger reduces the cost of coarse-filtering stage but makes each block a coarser proxy; smaller improves efficiency but increases the risk of missing relevant blocks. When and —the regime of ultra-long contexts with a selective coarse filter—the reduction is substantial. Conversely, as approaches , HISA degrades gracefully toward the DSA baseline.
As modern LLMs increasingly adopt context windows of 128K or even 1M tokens to support advanced agent capabilities and native multimodal reasoning, HISA’s asymptotic advantage translates directly into practical speedups.
5 Experiments
We evaluate HISA along five axes: (1) kernel-level latency, (2) retrieval accuracy on Needle-in-a-Haystack, (3) downstream task performance on LongBench, (4) visualization of attention scores, and (5) hyperparamenter sensitivity. Throughout the evaluation, we compare three indexing strategies:
-
•
DSA (original): the full-prefix token-level indexer as described in Section 3.
-
•
Block-Sparse: a block-level-only baseline that selects top- blocks and attends to all tokens within those blocks (i.e., Stage 1 only, without token-level refinement).
-
•
HISA: the hierarchical block-to-token indexer proposed in this work.
Both HISA and Block-Sparse are training-free: they are applied at inference time by replacing the indexer module, with no fine-tuning or architectural modification.
5.1 Kernel-Level Speedup
Figure 2 compares the indexer kernel latency of the original DSA and HISA across context lengths from 8K to 64K tokens. Both implementations use TileLang (Wang et al., 2025) kernels, with DSA following the official reference implementation.111https://github.com/tile-ai/tilelang/tree/main/examples/deepseek_v32 The HISA kernel is decomposed into two stages: block-level filtering and token-level refinement within the selected candidate blocks. The configuration is as follows: query lens , final top- tokens, block size , and two choices for the maximum number of selected blocks. All comparisons are conducted on an NVIDIA A100 GPU. These results are measured at the indexer kernel level and do not directly reflect end-to-end serving throughput, which also depends on the sparse MLA operator, KV cache management, and other system components.
With 2048 selected tokens, the sparse MLA operator consistently costs about 1.6 ms, while the indexer reaches 5.6 ms at 64K context length. This suggests that the main performance bottleneck in DSA lies in the indexer rather than in sparse MLA itself. Accordingly, we restrict the comparison to indexer overhead. At 64K context length, HISA delivers an approximately speedup with a 4:1 first-stage compression ratio (corresponding to a 16K candidate budget), and up to speedup under a fixed 8K budget. Although HISA adds a block-level filtering stage, this stage operates only on pooled block summaries of size , which is far smaller than the full token sequence. Moreover, under a fixed 8K budget, the second-stage cost remains nearly constant because both the input and output lengths are fixed, making the computation graph easier to optimize and further improving inference speed.
5.2 Needle-in-a-Haystack
The Needle-in-a-Haystack (NIAH) test (Kamradt, 2023) evaluates a model’s ability to retrieve a specific fact (the ”needle”) embedded at a controlled position within a long distractor context (the ”haystack”). We evaluate DeepSeek-V3.2 with its original DSA indexer replaced by HISA (4:1 ratio) and block indexer, without any additional training, over context lengths ranging from 8K to 648K tokens and needle insertion depths ranging from 0% (beginning) to 100% (end).
Figure 3 presents the retrieval accuracy heatmaps. The original DSA achieves near-perfect retrieval across all context lengths and needle positions (Figure 3(a)). HISA closely matches this performance (Figure 3(c)), with only marginal degradation at extreme lengths and depths, suggesting that the our HISA rarely discards blocks containing the target information. In contrast, the Block-Sparse baseline (Figure 3(b)) exhibits noticeable accuracy degradation, particularly when the needle is located in the middle of the context where block-level selection is least reliable. This result underscores the value of hierarchical selection. Block-sparse methods often waste budget on unimportant tokens within selected blocks while overlooking truly critical tokens. HISA, in contrast, refines the selection at the token level after block retrieval, allowing it to preserve important tokens more accurately and achieve efficient token-wise sparsity.
5.3 LongBench Evaluation
LongBench (Bai et al., 2024) is a comprehensive benchmark for long-context understanding, covering single-document QA, multi-document QA, summarization, few-shot learning, and synthetic retrieval tasks. We evaluate DeepSeek-V3.2 (DeepSeek-AI, 2025) and GLM-5 (GLM-5-Team, 2026) under three configurations: the original DSA indexer, HISA, and Block-Sparse Attention. For a fair comparison, all three configurations ultimately retain 2048 tokens for computation. Specifically, Block-Sparse Attention directly selects 16 blocks of size 128 (i.e., tokens). HISA first selects 64 blocks of size 128 (i.e., tokens), and then further refines them through token-level selection to 2048 tokens.
| Model | Indexer | SQA | MQA | Sum | FS | Syn | Code | Avg. |
|---|---|---|---|---|---|---|---|---|
| DeepSeek-V3.2 | DSA | 50.89 | 52.66 | 22.11 | 62.24 | 69.83 | 48.56 | 51.05 |
| Block | 48.36 | 49.76 | 21.90 | 59.45 | 68.67 | 49.09 | 49.54 | |
| HISA | 49.17 | 51.96 | 22.13 | 61.62 | 70.83 | 48.99 | 50.78 | |
| GLM-5 | DSA | 41.23 | 27.89 | 18.39 | 63.20 | 68.84 | 56.53 | 46.01 |
| Block | 38.35 | 24.29 | 16.95 | 60.64 | 60.49 | 55.29 | 42.67 | |
| HISA | 42.45 | 27.62 | 17.90 | 63.78 | 69.35 | 56.79 | 46.32 |
Table 1 summarizes the results. Across both models and all task categories, HISA achieves performance very close to that of the original DSA. Notably, HISA consistently surpasses DSA on the Synthetic tasks, and on GLM-5 it even attains a higher average score. By contrast, the Block-Sparse baseline, which does not include token-level refinement, exhibits a substantially larger performance gap. This is particularly apparent on the Synthetic tasks for GLM-5, where its score declines by 8.35%.
5.4 Visualization of Attention Scores
To analyze the structural properties of attention in long-context generation, we conduct a visualization study on a representative sample from the code task of LongBench. We generate the first output token using DeepSeek-V3.2 and extract the full attention distributions at each layer. We visualize the attention weights over all context tokens as a 2D heatmap, where the x-axis denotes token positions and the y-axis denotes layer indices.
The visualization reveals a pattern: tokens with high attention weights tend to form contiguous spans rather than appearing as isolated points in a considerable number of tasks. These high-density regions often correspond to semantically coherent segments (e.g.,code blocks,mathematical formulas and derivations) and persist across multiple layers. Outside these spans,attention scores are negligible. This observation suggests that attention mass may be naturally concentrated in block-wise regions. Therefore,block-level sparsification can retain most of the informative attention distribution while avoiding the fine-grained selection overhead of token-wise top-k methods. The results provide empirical support for the two-stage hierarchical structure of HISA.
5.5 Hyperparameter Sensitivity
We investigate the sensitivity of HISA to its two key hyperparameters—block size and block-level top-—by comparing three HISA configurations that share the same candidate pool size, , but different coarse-to-fine trade-offs: , , and . We further include the original DSA as an upper bound and Block-Sparse as a lower bound. All configurations use for the final token selection. Results are evaluated on DeepSeek-V3.2 and GLM-5 across five LongBench task categories.
Figure 5 reveals several key findings. First, all three HISA configurations closely track DSA performance across all five task categories. This result confirms that our two-stage hierarchical indexer recovers nearly the same set of important tokens as the exhaustive flat scan. Second, among the three HISA variants, the intermediate configurations ( and ) perform better than . This suggests that finer-grained selection is important for accurately identifying the most relevant tokens. Third, Block-Sparse consistently underperforms all HISA configurations. This gap underscores the importance of token-level refinement: even under the same block-level selection mechanism, the ability to prune low-relevance tokens within selected blocks yields measurable quality gains.
6 Conclusion and Future Directions
To address the emerging bottleneck caused by the complexity of the DSA indexer, we propose HISA, a hierarchical indexing approach. Specifically, HISA first uses a hardware-friendly block indexer to efficiently filter out a large number of irrelevant tokens, and then applies token-level reranking over the remaining candidates to construct the final cache for sparse attention computation. At the kernel level, HISA delivers a speedup over the DSA kernel. As a plug-and-play module, HISA can directly replace the token indexer in DeepSeek-V3.2 and GLM-5. Without any additional training, it maintains nearly unchanged performance on LongBench. On NIAH, it also performs significantly better than the corresponding block-sparse baseline.
Several avenues remain open: (1) Reducing information loss in coarse filtering: the current block-level stage represents each block with a single pooled vector, which can fail when a block crosses a semantic boundary and the pooled representation does not reflect the most important token. Potential mitigations include overlapping blocks, adaptive block boundaries, or replacing mean pooling with max pooling to better preserve salient outlier directions. (2) Training-aware HISA: while HISA currently operates as a training-free inference-time replacement, jointly training the block scoring stage may improve the coarse filter’s accuracy, particularly for such boundary cases. (3) End-to-end system integration: integrating HISA into a full inference serving stack (e.g., with continuous batching and speculative decoding) and measuring throughput and latency under realistic workloads.
References
- External Links: Link Cited by: §1.
- IndexCache: accelerating sparse attention via cross-layer index reuse. arXiv preprint arXiv:2603.12201. Cited by: §2.
- LongBench: a bilingual, multitask benchmark for long context understanding. arXiv preprint arXiv:2308.14508. Cited by: §5.3.
- FlashAttention: fast and memory-efficient exact attention with io-awareness. In Advances in Neural Information Processing Systems, Vol. 35. Cited by: §1.
- FlashAttention-2: faster attention with better parallelism and work partitioning. arXiv preprint arXiv:2307.08691. Cited by: §1.
- DeepSeek-V3 technical report. arXiv preprint arXiv:2412.19437. Cited by: §1.
- DeepSeek-v3.2: pushing the frontier of open large language models. arXiv preprint arXiv:2512.02556. Cited by: §1, §2, §3, §5.3.
- LazyLLM: dynamic token pruning for efficient long context LLM inference. arXiv preprint arXiv:2407.14057. Cited by: §2.
- GLM-5: from vibe coding to agentic engineering. arXiv preprint arXiv:2602.15763. Cited by: §1, §5.3.
- External Links: Link Cited by: §1.
- MInference 1.0: accelerating pre-filling for long-context llms via dynamic sparse attention. arXiv preprint arXiv:2407.02490. External Links: Link Cited by: §2.
- Needle in a haystack — pressure testing llms. Note: https://github.com/gkamradt/LLMTest_NeedleInAHaystack Cited by: §5.2.
- FlexPrefill: a context-aware sparse attention mechanism for efficient long-sequence inference. In International Conference on Learning Representations, Cited by: §2.
- Twilight: adaptive attention sparsity with hierarchical top- pruning. In Advances in Neural Information Processing Systems, Vol. 38. Cited by: §2.
- MoBA: mixture of block attention for long-context llms. arXiv preprint arXiv:2502.13189. Cited by: §1, §2, §4.1.
- External Links: Link Cited by: §1.
- MiniMax-01: scaling foundation models with lightning attention. arXiv preprint arXiv:2501.08313. Cited by: §1.
- Kimi K2: open agentic intelligence. arXiv preprint arXiv:2507.20534. Cited by: §1.
- Double-p: hierarchical top-p sparse attention for long-context LLMs. arXiv preprint arXiv:2602.05191. Cited by: §2.
- External Links: Link Cited by: §1.
- Transformers are multi-state RNNs. In Proceedings of the 2024 Conference on Empirical Methods in Natural Language Processing, Cited by: §2.
- External Links: Link Cited by: §1.
- TileLang: a composable tile-based programming model for ai systems. arXiv preprint arXiv:2504.17577. Cited by: §5.1.
- XAttention: block sparse attention with antidiagonal scoring. In Proceedings of the 42nd International Conference on Machine Learning, Cited by: §2.
- Hierarchical attention networks for document classification. In Proceedings of the 2016 Conference of the North American Chapter of the Association for Computational Linguistics: Human Language Technologies, pp. 1480–1489. Cited by: §2.
- Native sparse attention: hardware-aligned and natively trainable sparse attention. In Proceedings of the 63rd Annual Meeting of the Association for Computational Linguistics (Volume 1: Long Papers), pp. 23078–23097. Cited by: §1, §2, §2.
- SnapKV: llm knows what you are looking for before generation. arXiv preprint arXiv:2404.14469. External Links: Link Cited by: §2.
- SpargeAttention: accurate and training-free sparse attention accelerating any model inference. In Proceedings of the 42nd International Conference on Machine Learning, Cited by: §2.
- H2O: heavy-hitter oracle for efficient generative inference of large language models. In Advances in Neural Information Processing Systems, Vol. 36. Cited by: §2.
- InfLLM-v2: dense-sparse switchable attention for seamless short-to-long adaptation. In International Conference on Learning Representations, Cited by: §2.
Appendix A Algorithm Pseudocode
Algorithm 1 provides the complete pseudocode for the HISA indexer.
Appendix B Experimental Settings
We detail the experimental settings for long-context evaluations in this section. All evaluations were conducted in a zero-shot setting.
B.1 Long-context Benchmarks
We evaluated the long-context performance using the Needle In A Haystack (NIAH) test and the LongBench benchmark. We tested two models: DeepSeek-V3.2 and GLM-5. Both models were deployed using the vLLM online serving framework with FP8 precision.
NIAH Settings
For the NIAH experiments, we utilized a customized evaluation codebase modified from the RULER222https://github.com/NVIDIA/RULER GitHub repository. We did not apply chat templates to either model to ensure a direct assessment of their raw retrieval capabilities.
LongBench Settings
We evaluated LongBench using the lm-eval333https://github.com/EleutherAI/lm-evaluation-harness framework. The configurations for LongBench varied slightly depending on the model characteristics:
-
•
Chat Template Usage: DeepSeek-V3.2 was evaluated with its standard chat template. In contrast, GLM-5 was evaluated without a chat template. This decision was made because using the template triggered an extended thinking process that exceeded the maximum generation length and significantly slowed down inference. Furthermore, disabling the thinking process while keeping the template resulted in inferior performance compared to not using the template at all.
-
•
Concurrency Settings: The default number of concurrent requests (num_concurrent) was set to 20. However, due to Out-Of-Memory (OOM) issues specific to GLM-5 on certain tasks, we adjusted the concurrency: longbench_single was run with a concurrency of 1, and longbench_summary was run with a concurrency of 2.
Fairness of Comparison
We emphasize that although the specific settings (e.g., concurrency, chat template) differ across models and tasks to accommodate their unique characteristics and hardware constraints, we ensure that the settings are strictly aligned when comparing different methods within the same model and task combination. This guarantees a fair and rigorous comparison.

---

# HISA: Efficient Hierarchical Indexing for Fine-Grained Sparse Attention - https://arxiv.org/pdf/2603.18815
Source URL: https://arxiv.org/pdf/2603.18815

Source Type: pdf

Source ID: d716eba5-9e3d-4df3-a3d8-3fa9697a3d84


%PDF-1.7
%┐„ó■
1 0 obj
<< /Metadata 3 0 R /Names 4 0 R /OpenAction 5 0 R /Outlines 6 0 R /PageMode /UseOutlines /Pages 7 0 R /Type /Catalog >>
endobj
2 0 obj
<< /Author (Hao Zhang; Mingjie Liu; Shaokun Zhang; Songyang Han; Jian Hu; Zhenghui Jin; Yuchi Zhang; Shizhe Diao; Ximing Lu; Binfeng Xu; Zhiding Yu; Jan Kautz; Yi Dong) /Creator (arXiv GenPDF \(tex2pdf:a6404ea\)) /DOI (https://doi.org/10.48550/arXiv.2603.18815) /License (http://creativecommons.org/licenses/by/4.0/) /PTEX.Fullbanner (This is pdfTeX, Version 3.141592653-2.6-1.40.28 \(TeX Live 2025\) kpathsea version 6.4.1) /Producer (pikepdf 8.15.1) /Title (ProRL Agent: Rollout-as-a-Service for RL Training of Multi-Turn LLM Agents) /Trapped /False /arXivID (https://arxiv.org/abs/2603.18815v1) >>
endobj
3 0 obj
<< /Subtype /XML /Type /Metadata /Length 1896 >>
stream
ProRL Agent: Rollout-as-a-Service for RL Training of Multi-Turn LLM AgentsHao ZhangMingjie LiuShaokun ZhangSongyang HanJian HuZhenghui JinYuchi ZhangShizhe DiaoXiming LuBinfeng XuZhiding YuJan KautzYi Donghttp://creativecommons.org/licenses/by/4.0/cs.AI
endstream
endobj
4 0 obj
<< /Dests 8 0 R >>
endobj
5 0 obj
<< /D [ 9 0 R /Fit ] /S /GoTo >>
endobj
6 0 obj
<< /Count 6 /First 10 0 R /Last 11 0 R /Type /Outlines >>
endobj
7 0 obj
<< /Count 22 /Kids [ 12 0 R 13 0 R 14 0 R 15 0 R ] /Type /Pages >>
endobj
8 0 obj
<< /Kids [ 16 0 R 17 0 R 18 0 R 19 0 R 20 0 R ] /Limits [ (Doc-Start) (table.caption.7) ] >>
endobj
9 0 obj
<< /Annots [ 21 0 R 22 0 R 23 0 R 24 0 R 25 0 R 26 0 R 27 0 R 28 0 R 29 0 R 30 0 R 31 0 R 32 0 R 33 0 R 34 0 R 35 0 R 36 0 R 37 0 R 38 0 R 39 0 R 40 0 R 41 0 R 42 0 R 43 0 R 44 0 R 45 0 R 46 0 R 47 0 R 48 0 R 49 0 R 50 0 R 51 0 R 52 0 R 53 0 R 54 0 R ] /Contents [ 55 0 R 56 0 R 57 0 R 58 0 R ] /Group 59 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 12 0 R /Resources 60 0 R /Type /Page >>
endobj
10 0 obj
<< /A 61 0 R /Next 62 0 R /Parent 6 0 R /Title 63 0 R >>
endobj
11 0 obj
<< /A 64 0 R /Parent 6 0 R /Prev 65 0 R /Title 66 0 R >>
endobj
12 0 obj
<< /Count 6 /Kids [ 9 0 R 67 0 R 68 0 R 69 0 R 70 0 R 71 0 R ] /Parent 7 0 R /Type /Pages >>
endobj
13 0 obj
<< /Count 6 /Kids [ 72 0 R 73 0 R 74 0 R 75 0 R 76 0 R 77 0 R ] /Parent 7 0 R /Type /Pages >>
endobj
14 0 obj
<< /Count 6 /Kids [ 78 0 R 79 0 R 80 0 R 81 0 R 82 0 R 83 0 R ] /Parent 7 0 R /Type /Pages >>
endobj
15 0 obj
<< /Count 4 /Kids [ 84 0 R 85 0 R 86 0 R 87 0 R ] /Parent 7 0 R /Type /Pages >>
endobj
16 0 obj
<< /Kids [ 88 0 R 89 0 R 90 0 R 91 0 R 92 0 R 93 0 R ] /Limits [ (Doc-Start) (cite.wang2024openhands) ] >>
endobj
17 0 obj
<< /Kids [ 94 0 R 95 0 R 96 0 R 97 0 R 98 0 R 99 0 R ] /Limits [ (cite.wang2025vagen) (lstnumber.1.5) ] >>
endobj
18 0 obj
<< /Kids [ 100 0 R 101 0 R 102 0 R 103 0 R 104 0 R 105 0 R ] /Limits [ (lstnumber.1.6) (page.11) ] >>
endobj
19 0 obj
<< /Kids [ 106 0 R 107 0 R 108 0 R 109 0 R 110 0 R 111 0 R ] /Limits [ (page.12) (subsubsection.3.3.1) ] >>
endobj
20 0 obj
<< /Kids [ 112 0 R 113 0 R ] /Limits [ (subsubsection.3.3.2) (table.caption.7) ] >>
endobj
21 0 obj
<< /A << /S /ResetForm >> /AP << /N << /On 114 0 R >> >> /AS /On /F 4 /FT /Btn /Ff 65537 /Rect [ .996 837.893 3.996 840.893 ] /Subtype /Widget /T (pbs@ARFix@1) /Type /Annot >>
endobj
22 0 obj
<< /A << /S /URI /Type /Action /URI (https://github.com/NVIDIA-NeMo/ProRL-Agent-Server) >> /Border [ 0 0 0 ] /C [ 0 1 1 ] /H /I /Rect [ 221.536 494.564 281.103 506.24 ] /Subtype /Link /Type /Annot >>
endobj
23 0 obj
<< /A << /D (cite.cao2025skyrl) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 338.481 428.487 380.427 440.068 ] /Subtype /Link /Type /Annot >>
endobj
24 0 obj
<< /A << /D (cite.cao2025skyrl) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 383.787 428.487 412.444 440.068 ] /Subtype /Link /Type /Annot >>
endobj
25 0 obj
<< /A << /D (cite.gao2025beyond) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 416.205 428.487 458.756 440.068 ] /Subtype /Link /Type /Annot >>
endobj
26 0 obj
<< /A << /D (cite.gao2025beyond) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 462.116 428.487 485.822 440.068 ] /Subtype /Link /Type /Annot >>
endobj
27 0 obj
<< /A << /D (cite.guo2025deepseek) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 489.583 428.487 532.58 440.068 ] /Subtype /Link /Type /Annot >>
endobj
28 0 obj
<< /A << /D (cite.guo2025deepseek) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 61.087 414.738 85.347 426.32 ] /Subtype /Link /Type /Annot >>
endobj
29 0 obj
<< /A << /D (cite.hu2025openreasonerzero) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 89.324 414.738 128.719 426.32 ] /Subtype /Link /Type /Annot >>
endobj
30 0 obj
<< /A << /D (cite.hu2025openreasonerzero) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 132.286 414.738 156.546 426.32 ] /Subtype /Link /Type /Annot >>
endobj
31 0 obj
<< /A << /D (cite.luo2025deepswe) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 160.524 414.738 202.973 426.32 ] /Subtype /Link /Type /Annot >>
endobj
32 0 obj
<< /A << /D (cite.luo2025deepswe) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 206.54 414.738 235.876 426.32 ] /Subtype /Link /Type /Annot >>
endobj
33 0 obj
<< /A << /D (cite.jimenez2023swe) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 349.983 400.99 411.738 412.572 ] /Subtype /Link /Type /Annot >>
endobj
34 0 obj
<< /A << /D (cite.jimenez2023swe) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 415.293 400.99 439.42 412.572 ] /Subtype /Link /Type /Annot >>
endobj
35 0 obj
<< /A << /D (cite.zhou2023webarena) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 509.414 400.99 533.91 412.572 ] /Subtype /Link /Type /Annot >>
endobj
36 0 obj
<< /A << /D (cite.zhou2023webarena) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 61.366 387.242 84.566 398.823 ] /Subtype /Link /Type /Annot >>
endobj
37 0 obj
<< /A << /D (cite.zhou2023webarena) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 88.074 387.242 111.958 398.823 ] /Subtype /Link /Type /Annot >>
endobj
38 0 obj
<< /A << /D (cite.xie2024osworld) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 298.663 387.242 337.983 398.823 ] /Subtype /Link /Type /Annot >>
endobj
39 0 obj
<< /A << /D (cite.xie2024osworld) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 341.491 387.242 365.375 398.823 ] /Subtype /Link /Type /Annot >>
endobj
40 0 obj
<< /A << /D (cite.cao2025skyrlagent) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 456.304 284.129 498.917 295.71 ] /Subtype /Link /Type /Annot >>
endobj
41 0 obj
<< /A << /D (cite.cao2025skyrlagent) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 502.445 284.129 531.717 295.71 ] /Subtype /Link /Type /Annot >>
endobj
42 0 obj
<< /A << /D (cite.jiang2025verltool) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 60.927 270.381 110.322 281.962 ] /Subtype /Link /Type /Annot >>
endobj
43 0 obj
<< /A << /D (cite.jiang2025verltool) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 113.869 270.381 138.018 281.962 ] /Subtype /Link /Type /Annot >>
endobj
44 0 obj
<< /A << /D (cite.liu2025gem) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 141.973 270.381 181.814 281.962 ] /Subtype /Link /Type /Annot >>
endobj
45 0 obj
<< /A << /D (cite.liu2025gem) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 185.361 270.381 214.88 281.962 ] /Subtype /Link /Type /Annot >>
endobj
46 0 obj
<< /A << /D (cite.luo2025agentlightning) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 218.835 270.381 261.097 281.962 ] /Subtype /Link /Type /Annot >>
endobj
47 0 obj
<< /A << /D (cite.luo2025agentlightning) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 264.643 270.381 293.236 281.962 ] /Subtype /Link /Type /Annot >>
endobj
48 0 obj
<< /A << /D (cite.sheng2025verl) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 297.191 270.381 350.133 281.962 ] /Subtype /Link /Type /Annot >>
endobj
49 0 obj
<< /A << /D (cite.sheng2025verl) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 353.679 270.381 377.829 281.962 ] /Subtype /Link /Type /Annot >>
endobj
50 0 obj
<< /A << /D (cite.tan2025rllm) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 381.784 270.381 423.846 281.962 ] /Subtype /Link /Type /Annot >>
endobj
51 0 obj
<< /A << /D (cite.tan2025rllm) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 427.393 270.381 451.542 281.962 ] /Subtype /Link /Type /Annot >>
endobj
52 0 obj
<< /A << /D (cite.xi2026agentgymrl) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 455.497 270.381 490.466 281.962 ] /Subtype /Link /Type /Annot >>
endobj
53 0 obj
<< /A << /D (cite.xi2026agentgymrl) /S /GoTo >> /Border [ 0 0 0 ] /C [ 0 1 0 ] /H /I /Rect [ 494.013 270.381 518.162 281.962 ] /Subtype /Link /Type /Annot >>
endobj
54 0 obj
<< /A << /S /URI /URI (https://arxiv.org/abs/2603.18815v1) >> /BS << /W 0 >> /NM (fitz-L0) /Rect [ 12 248.195 32 593.695 ] /Subtype /Link >>
endobj
55 0 obj
<< /Length 10 /Filter /FlateDecode >>
stream
x£+õ Ņ |
endstream
endobj
56 0 obj
<< /Filter /FlateDecode /Length 4111 >>
stream
x┌š[Ys█F~„»Ó#X%┬śW÷╔ēŪ^{ūk+ē│N@&æ ĆVõ_┐}
.A┘╚Q¬vKU0gOO„ūŪā┼n,×=
õ®Ó░Pŗ(¶Ģčŗ8 |ķ┼µ°ĶūG~ÉĶ╚Pā┴+U® ķK½└Ģ>~~Tŗ¦šŻ┴_0Ö&C|ā Ž░ĘL║ę~/VFĘ8┬¹¤ā┼j^@╦4ŹūįŅĖ░1¶▒ūa±vvtķ“c"]5╬5){¾ūĘ’ų%▄\
ž∙ÕÕŻŪ▀(xWæ¤Ü(\\~XDÜjck}ź═Ōr╗x’Į«½7/Ś+Ø*’╔./█/¢+c’═■Uć├R)»Z¬└;Ę½l®»Ye½Ę╦UĻÕ§Ūb®¼Ę╔╣„ćźų^U¾ć“▓╬Ŗ▓(w³U-uĒ~Šå├fh?U¢®xu>┤┼jib’“\Ś▄·Õ╦W║ņųqłŚ¢°iY\ņ_Ō$ŗ%ēoĮĖ<┬▀"┴Yģ├▐┐Ś+e╝}T┴ÉÅ┐üŲĪ¤Ųqä9¢¬ ÷Ģ,óT¹@¾SŅsńņJ)▀śp<²»Ń1"č▐/└Ėą╦Ö£Ś┼∙„ĶĆÕ─0╬ĮķłĆ■4ØźŃĒR%░■ĻĻī£Nę?ŲmB?RßĮ)č&4wP▓ŖQ╝īWŅn2 v.+źčŗb╣ŖÓS*ŽR,DńÕn.śĢ/
ūńŪźU▐y®"o│/·EZZdŽåį├Zśņė^vŃ)OV]Éö¬^”ąĮ(6~ś─╝äw┼▒ņŚÄ©/ŗ“ĒŁ½y7!ĘžvØ~Dsš/P52æ³┐g¼q¤║š©ž+°²®░kF(T©ł ½└ćŚĢ│1½žōuėų┘”ØķķH÷Ē¢"┴░!ų;=mEG¦Ż░[¼Ż³^ń╠“óDZ7u╬╚+?▄pōŌx¬Ļ▐q¤[. 4«ģSüĘ®åÄ¦▓% ĒBŲ(█T,#’ŃRGNĪdXÜY|šHć¼▄“KØŃ6člø|┼xūŃģćiŌą║Å9Si<”!½¬iąü0Ć7
Ā»Ŗŗåøg³Ė╩o°zš∙¢ĆSåį├ģõL
8Öx╗ ┌}^įā┘EV¹¬ķEÖÕRnČ╬„ŖØŌą K×Å╚«A╗Ā╠Ézū─4f6č¼¢Fą█xŁCoŪĪp┬Ī─°ē▒╠ó: §\įy├b
ŌĆ¾└Ą▌:żüYĮŗQ"qńŃ:»øæ}ĀŲ
Ņ¢Ź╝u§[ŠÕóÜņQÓņ„╔A¬Łć▄@Č6ó5▓▌$”h┌NĒ`ļkÄČfä q╬G┤@<®┐Le"J─D(6Lłł%p7┤Ęi─B│}Ä┌▓
ąļv
½«ŗvÅowÖŗ:«S±a XØ`M&╝cå\IŁ§Ü©n┌³ž„>½Ę2räÅ┼”╬╣£ĖBź0ą[ /■]ļ
┴│░÷▀Īżö[ÉĢĢčŖfĶÅėä╝¦╩ └wĆēąwh~Cg8”ĮO¹éųĮÖl¬ė■8”P:
<«Eū®!ņĻÅ^▌hżžz»Ś½ųēé’oĶ?HĖIS’ } 3c▐K·▀▐Aī(TdL@│ĪčŲū5Æūēävć╦
£Ńaų:ĒÜÓnÓāŲļæk(Ü$¦a─┬~ iKņØü┘¶I0╠ņÕé┴>(ų(<lż67G(Ę¹ÜĘkĘŚčJ~>yŹĆ¶£?ĶŽ¾]¦CŠ█[|W„µ;#)¶;4Sčüõ¢Yt9s▓Ķp³įŁōme&ųæom╩f┤H@P ”X#╗;Ņ@9RłĆ$
┐>5ļ¾Q įą«r¦¾I]{@U(¢¬ndį±åAŻæ§2)X/nXö\_WU╦d5R¾-ē└ļ»ĖŠ╔[─║čś¹├2BŽp@f*~>Ō©,═┼±b^®p«▀Q¬P▀[®t2=XÖ de@$=ręB+õõÓž°kóĮ╬▌╬ņž§╚¾║`JÅY╗ŚūĘŚD¹ū».x¼n¾7šČøL└æ4Ś²łYyWcĪ▓iņŪĪ°ł=Ż┬█īŖŅ═(ł_åĻöŚ½åyN╠VL×xw0~$═RĘB└æć(m─ś*“║╚£Ė)YRķ²öšĒįÆ+’▀?' }Ŗ>Ū¾'RH”·U┼_ŽnÄ■hų;[łŹ±Ż e¦V∙Sb├ ×Śm]mæ┴█Ī┘ØĖ┐®╗┐Č6¶Ł¾▀`TæožqC┘ž▓╬öć
ŚÆ”┬S╝LC^µ▒ļ4q ĪõC═■-~% Ŗ¤S,é8F#hUń"“[ÖĮPžEĆ▐’▀łŖC¾³nµü šŚ╗3ģŪł¦«Ļłš6?F_┐ķGFeø7\ų(?F ¬Å bxŪ%’ŗŌśA ▒AbiKvßF8¦_ē┬a4+ĘäĀŲP═0ĀB9╣Ē„ė’½LfŽ[7#“q*ä▐┼ĮšŲęü│ķH╗ Hü„lL”C§ć 3 2ĆJP·3åĄ3r@ķ0L]0łUé─,¼Ŗ²Tŗ'¹@kłĮoŽHv"õŪ╚Ķ°ßł|I(X²Ąö▐Æ\ä
tBtłüCņĢćŌ*ń╣gt{Ō°żĄC∙æp|©ĶH<öēóV¶╔ ØóS╣Ą▐═ēAüū═(>ź╣ĆvxäĶa╗h7ØÓZ`}lÆ|`¶J%ÉėI@$¬öńbl0ń.^pÅF2U▄░│©■H╬mĻ³─ßP1Ŗu╣┘ń`┘ŗŌ(1·'!░ÕÖN2Rö3'▓pÕų½5{¾ū╦├ŗŽ_ō$┘ž-Ļ╠ė─>Özk ¼xčĻßØ┐┐h┼µ×Æy)∙k╚┴¦KyØ█\ÜāŪū%SPfæ?V╗▐▒ 9='Ųż%dęņ'x”Å£kÕÅóźÕ
syē³Jxr^b¶IśÆxOG.=Ļ╝īCt|3═▀ę╬Ūx&Æ╩ņS=äŗ¦æŪśsßLrI*8┬ŗōķ0ä<¦LŠČ┤ŗĀ_ō4├2 Ü│āŹe¾-ØPćł╬Ą@*»Ā¾\.'ł╠"?%[wē▒e¤hR╔ äģ=é░
K×
┌Įy╔Ž>▀g╚┘=Õ į"ĖRŗĻpĶå’bą·TŹ;”q£▒)låŪ°ŖĢ▄Ŗ▒▒å¾Xś@D¢tėHRj&Ģe,<āą%+Q\ĖÜł╣}ŲŖ.5ŗš®-Ä·╩¤žsŅŌ
ĒĢ ¢─gO©ct█±]║(źņP╔:ĀjÆõ.ąl’å+w ĖL“J:40FŻ┤½µ┤ļ(šc¼sÖ┬-.ļ,▌TOtŌć.å=f┐T╔ļ¬Ģ▄Ä═Ś;┼1żČPÓ³tįÓĮöĒsą╩Õģ{iŠ█:Ŗ9d┼H1]ņ▓▄½ólå¦Jb░±░ ×)@¤<ń─MµĶ-╩}^s@¢»┼ķ╚®└Ķß.µ(ļ'üdŹn¹O®„═ę$=ę ¢╣}ĮĀ¢vÄÉbĖø╦p#2Ŗģhj`„š(ßĪ” A┴9(Q#─ŌßG.P─ 5Ū¼╝æ®AūixdŪżņūą▄ĘvŅÄ└ąĀĢč─)Ą^I▐PXF×Pś÷’ū{▐®:ÆÖ║ßē2Ž╚Ņ(Ć©rĘ?HĶuO§┌g=9
AŚsÄŌoxī-ÓY)╣ óDO╬ĪÆµø¾p\╠ž"²ä$Ča¢IS∙ęIņź;£"ā┬▌─°ź▐¾▓│zŻĻ▒t§ń+▌Ų'─N©╗*'ZÆnjöt├Ė<;µū$Ąō!9^óö┤ł.ÕE"0∙1»wö"·Lūä#Ē▒[“@Ü”xe²'C½nūäĢvĪ±h8ŃĻN¼w├É+y@Ū:y©Ó0Īļ)Ģļć!¾v¹żnäTŠ^æ _ņ■Õ2 <ńŁ■OS·«°KēīµėŅ▄d`╔k:į~ĻÄE1Ż¶▀ mč¼üm;ŃP&Č`QÜbWr█B×āōihqh▒e├uŌF»ŃŲ0ź▌ ┤īü¦d:Üż@═└ŁH┘½Ć2╔W─Ų
m■ŃÖc;▒P&§U*ēńćm«─°ĖDłĻ*÷│µN╔ö╣Śm¹Až’qż3K|½Ćn+ąWÓ]Ņŗf4╗∙”sā╚_BČŖ_ātI┤7hHŚh╝▄fÆĖwigÓČś¤i=Ą3µSC«C▀Z;k╚ŗuq(┌QH)ń.┤ļR▌╔tqę¹õxVw,ä└Ī╦▀L4C)?R±┌∙æų│ć'ė'ö{Ę¾Iõ¦ßBÓ·jqMŠĻ’n+"¢ĘČ;=ļ“rl╔č"∙§_╚±Vjo³wūq
Ŗ”a9▐„└Uē-╣%jĀu|>▌╣æą`║æT╗šŁM├IpŽtö.T
e.╚AÆ~Æą+Ņ\ŲAą└6n÷·ĮĆŹĶ(å!^ÜY;ÄēęŹ=¤?■'╝D▐
}g!ŲŚÉb
r.3ķyÄmĪ█Ķ▄Ü(©¾Qõ*▌GWŗ²Q§¢{╣L╠¶▓öcG^½Ņn+4═P%█'¦¼Ņ╠åĮ÷Į°─┬│A°å
]š█Bś8Ähi└
5╬+ģ}«¾Łhs=2ą├¹aė┼æDL▐Hzķ7CdK╚_▀īŲj▌-óŗ╗$$Ś0vaDTĶ={ŹØŠø█CT¶ĪAĘ▄Ė_kĶ2ŅįÅģŻ█½¼wŌāļå»C§g▄ĪĘ½│ß4(ÖnEw{böõ0öõ0ał¬▒1a╦3?ĘxøT╚╣®$t«£yciļąnōu;’É"─N1yw&®2dT¤▓ļsb▌!¬©║k=Pj÷%@§Ē ┼│Óe#ŃMsźī»Sz─6a³į■╗KÖLśb"?×<-║╔%Zįµ├kwg?x┴X╩wČP^qÅPĀĮ╗ąV·├×é|½f«A!Śņ:OŖHLA“słĘ[ÆC++Ę║ĒįėĢ(B3ā▒C.ŃÆ9Ż9├%
÷ėłĢÓ;¼┼#K>eĻµRē├³D@gdk&ą▌ł£b┬█nÉBŗõI╗!Ų~t╩kĢ
q'SĢ╗Ü/_┼ķ╚Ł+w°AĆO°FūZ?K∙·r╦wū:“Ą·3┐LĪ_}ī«’╗╗n1`ā³žó┐┤Å┐”üŖ8PįįŹ┴╔$J~┬>^-p┘>Šw[å∙ļ;åĻgx° ▄³Ić¬GŁa5°╦ŹčäōøC>Ś>ķ 7░ķ└Ø«Hó├2O╦}¤░? ┌»ąŹ
endstream
endobj
57 0 obj
<< /Length 11 /Filter /FlateDecode >>
stream
x£Ń
õ ═ f
endstream
endobj
58 0 obj
<< /Filter /FlateDecode /Length 140 >>
stream
x┌EL9
A╠¹²üY¹ś─@┴╠e21X▌]1E²?8ŻéTįXg d4ī¶┴╣└ū2ŖŹ'ćFsüEŠ¢ķe·{n(äyŲ╬²žo ■y┴├ęs}
^§J¬2)k¼`uĄg!!w“Ļb┘&7Č”«I╚ÄĒĪ&č&┐:µl2ņß
ó×'
endstream
endobj
59 0 obj
<< /CS /DeviceRGB /I true /S /Transparency /Type /Group >>
endobj
60 0 obj
<< /ColorSpace 115 0 R /ExtGState 116 0 R /Font << /F162 117 0 R /F179 118 0 R /F184 119 0 R /F194 120 0 R /F210 121 0 R /F213 122 0 R /F62 123 0 R /Times-Roman 124 0 R >> /Pattern 125 0 R /ProcSet [ /PDF /Text /ImageC ] /XObject << /Im1 126 0 R >> >>
endobj
61 0 obj
<< /D (section.1) /S /GoTo >>
endobj
62 0 obj
<< /A 127 0 R /Next 128 0 R /Parent 6 0 R /Prev 10 0 R /Title 129 0 R >>
endobj
63 0 obj
endobj
64 0 obj
<< /D (appendix.A) /S /GoTo >>
endobj
65 0 obj
<< /A 130 0 R /Next 11 0 R /Parent 6 0 R /Prev 131 0 R /Title 132 0 R >>
endobj
66 0 obj
endobj
67 0 obj
<< /Annots [ 133 0 R 134 0 R 135 0 R 136 0 R 137 0 R 138 0 R 139 0 R 140 0 R 141 0 R 142 0 R 143 0 R 144 0 R ] /Contents 145 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 12 0 R /Resources 146 0 R /Type /Page >>
endobj
68 0 obj
<< /Annots [ 147 0 R 148 0 R 149 0 R 150 0 R 151 0 R 152 0 R 153 0 R 154 0 R 155 0 R 156 0 R 157 0 R 158 0 R 159 0 R 160 0 R 161 0 R 162 0 R 163 0 R 164 0 R 165 0 R 166 0 R 167 0 R 168 0 R 169 0 R 170 0 R 171 0 R 172 0 R 173 0 R 174 0 R 175 0 R 176 0 R 177 0 R 178 0 R 179 0 R 180 0 R 181 0 R 182 0 R 183 0 R 184 0 R 185 0 R 186 0 R 187 0 R 188 0 R 189 0 R 190 0 R 191 0 R 192 0 R 193 0 R 194 0 R 195 0 R 196 0 R 197 0 R 198 0 R 199 0 R 200 0 R 201 0 R 202 0 R 203 0 R 204 0 R 205 0 R 206 0 R 207 0 R ] /Contents 208 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 12 0 R /Resources 209 0 R /Type /Page >>
endobj
69 0 obj
<< /Annots [ 210 0 R 211 0 R 212 0 R 213 0 R 214 0 R 215 0 R 216 0 R 217 0 R 218 0 R 219 0 R 220 0 R ] /Contents 221 0 R /Group 222 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 12 0 R /Resources 223 0 R /Type /Page >>
endobj
70 0 obj
<< /Annots [ 224 0 R 225 0 R 226 0 R ] /Contents 227 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 12 0 R /Resources 228 0 R /Type /Page >>
endobj
71 0 obj
<< /Annots [ 229 0 R ] /Contents 230 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 12 0 R /Resources 231 0 R /Type /Page >>
endobj
72 0 obj
<< /Annots [ 232 0 R 233 0 R ] /Contents 234 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 13 0 R /Resources 235 0 R /Type /Page >>
endobj
73 0 obj
<< /Annots [ 236 0 R 237 0 R 238 0 R ] /Contents 239 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 13 0 R /Resources 240 0 R /Type /Page >>
endobj
74 0 obj
<< /Annots [ 241 0 R 242 0 R 243 0 R 244 0 R ] /Contents 245 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 13 0 R /Resources 246 0 R /Type /Page >>
endobj
75 0 obj
<< /Annots [ 247 0 R 248 0 R 249 0 R 250 0 R 251 0 R 252 0 R 253 0 R ] /Contents 254 0 R /Group 255 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 13 0 R /Resources 256 0 R /Type /Page >>
endobj
76 0 obj
<< /Annots [ 257 0 R 258 0 R 259 0 R 260 0 R 261 0 R 262 0 R 263 0 R 264 0 R 265 0 R 266 0 R 267 0 R 268 0 R 269 0 R 270 0 R ] /Contents 271 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 13 0 R /Resources 272 0 R /Type /Page >>
endobj
77 0 obj
<< /Annots [ 273 0 R 274 0 R 275 0 R 276 0 R 277 0 R 278 0 R 279 0 R 280 0 R 281 0 R 282 0 R 283 0 R 284 0 R ] /Contents 285 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 13 0 R /Resources 286 0 R /Type /Page >>
endobj
78 0 obj
<< /Annots [ 287 0 R 288 0 R 289 0 R 290 0 R 291 0 R 292 0 R ] /Contents 293 0 R /Group 59 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 14 0 R /Resources 294 0 R /Type /Page >>
endobj
79 0 obj
<< /Annots [ 295 0 R ] /Contents 296 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 14 0 R /Resources 297 0 R /Type /Page >>
endobj
80 0 obj
<< /Annots [ 298 0 R 299 0 R 300 0 R 301 0 R 302 0 R 303 0 R 304 0 R 305 0 R 306 0 R 307 0 R 308 0 R 309 0 R 310 0 R 311 0 R 312 0 R 313 0 R 314 0 R 315 0 R 316 0 R 317 0 R 318 0 R 319 0 R 320 0 R 321 0 R 322 0 R 323 0 R ] /Contents 324 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 14 0 R /Resources 325 0 R /Type /Page >>
endobj
81 0 obj
<< /Annots [ 326 0 R 327 0 R 328 0 R 329 0 R 330 0 R 331 0 R 332 0 R 333 0 R 334 0 R 335 0 R 336 0 R 337 0 R 338 0 R 339 0 R 340 0 R 341 0 R 342 0 R 343 0 R 344 0 R 345 0 R 346 0 R 347 0 R 348 0 R 349 0 R 350 0 R 351 0 R 352 0 R 353 0 R 354 0 R 355 0 R 356 0 R 357 0 R ] /Contents 358 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 14 0 R /Resources 359 0 R /Type /Page >>
endobj
82 0 obj
<< /Annots [ 360 0 R 361 0 R 362 0 R 363 0 R 364 0 R 365 0 R 366 0 R 367 0 R 368 0 R 369 0 R 370 0 R 371 0 R 372 0 R 373 0 R 374 0 R 375 0 R 376 0 R 377 0 R 378 0 R 379 0 R 380 0 R ] /Contents 381 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 14 0 R /Resources 382 0 R /Type /Page >>
endobj
83 0 obj
<< /Annots [ 383 0 R ] /Contents 384 0 R /Group 59 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 14 0 R /Resources 385 0 R /Type /Page >>
endobj
84 0 obj
<< /Annots [ 386 0 R ] /Contents 387 0 R /Group 59 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 15 0 R /Resources 388 0 R /Type /Page >>
endobj
85 0 obj
<< /Annots [ 389 0 R ] /Contents 390 0 R /Group 59 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 15 0 R /Resources 391 0 R /Type /Page >>
endobj
86 0 obj
<< /Annots [ 392 0 R ] /Contents 393 0 R /Group 59 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 15 0 R /Resources 394 0 R /Type /Page >>
endobj
87 0 obj
<< /Annots [ 395 0 R ] /Contents 396 0 R /Group 59 0 R /MediaBox [ 0 0 595.276 841.89 ] /Parent 15 0 R /Resources 397 0 R /Type /Page >>
endobj
88 0 obj
<< /Limits [ (Doc-Start) (Item.5) ] /Names [ (Doc-Start) 398 0 R (Item.1) 399 0 R (Item.2) 400 0 R (Item.3) 401 0 R (Item.4) 402 0 R (Item.5) 403 0 R ] >>
endobj
89 0 obj
<< /Limits [ (appendix.A) (cite.guo2025deepseek) ] /Names [ (appendix.A) 404 0 R (cite.cao2025skyrl) 405 0 R (cite.cao2025skyrlagent) 406 0 R (cite.deepscaler2025) 407 0 R (cite.gao2025beyond) 408 0 R (cite.guo2025deepseek) 409 0 R ] >>
endobj
90 0 obj
<< /Limits [ (cite.hu2025openreasonerzero) (cite.jin2025searchr1) ] /Names [ (cite.hu2025openreasonerzero) 410 0 R (cite.jain2025r2egym) 411 0 R (cite.jiang2025verltool) 412 0 R (cite.jimenez2023swe) 413 0 R (cite.jimenez2024swebench) 414 0 R (cite.jin2025searchr1) 415 0 R ] >>
endobj
91 0 obj
<< /Limits [ (cite.kaelbling1998pomdp) (cite.luo2025agentlightning) ] /Names [ (cite.kaelbling1998pomdp) 416 0 R (cite.kwon2025vllm) 417 0 R (cite.li2025torl) 418 0 R (cite.liu2025gem) 419 0 R (cite.lu2025scp116khighqualityproblemsolutiondataset) 420 0 R (cite.luo2025agentlightning) 421 0 R ] >>
endobj
92 0 obj
<< /Limits [ (cite.luo2025deepswe) (cite.ragenv2026collapse) ] /Names [ (cite.luo2025deepswe) 422 0 R (cite.nemo-gym) 423 0 R (cite.nemo-rl) 424 0 R (cite.patil2025the) 425 0 R (cite.prorl2025) 426 0 R (cite.ragenv2026collapse) 427 0 R ] >>
endobj
93 0 obj
<< /Limits [ (cite.shao2024deepseekmath) (cite.wang2024openhands) ] /Names [ (cite.shao2024deepseekmath) 428 0 R (cite.sheng2025verl) 429 0 R (cite.tan2025rllm) 430 0 R (cite.team2025notokenization) 431 0 R (cite.wang2024codeact) 432 0 R (cite.wang2024openhands) 433 0 R ] >>
endobj
94 0 obj
<< /Limits [ (cite.wang2025vagen) (cite.yu2025dapoopensourcellmreinforcement) ] /Names [ (cite.wang2025vagen) 434 0 R (cite.xi2026agentgymrl) 435 0 R (cite.xie2024osworld) 436 0 R (cite.yang2024sweagent) 437 0 R (cite.yao2022react) 438 0 R (cite.yu2025dapoopensourcellmreinforcement) 439 0 R ] >>
endobj
95 0 obj
<< /Limits [ (cite.yuan2024implicitprm) (figure.caption.1) ] /Names [ (cite.yuan2024implicitprm) 440 0 R (cite.zhang2024offline) 441 0 R (cite.zhang2026nemotronresearchtooln) 442 0 R (cite.zheng2024sglang) 443 0 R (cite.zhou2023webarena) 444 0 R (figure.caption.1) 445 0 R ] >>
endobj
96 0 obj
<< /Limits [ (figure.caption.10) (figure.caption.3) ] /Names [ (figure.caption.10) 446 0 R (figure.caption.11) 447 0 R (figure.caption.12) 448 0 R (figure.caption.13) 449 0 R (figure.caption.14) 450 0 R (figure.caption.3) 451 0 R ] >>
endobj
97 0 obj
<< /Limits [ (figure.caption.4) (lstlisting.2) ] /Names [ (figure.caption.4) 452 0 R (figure.caption.6) 453 0 R (figure.caption.8) 454 0 R (figure.caption.9) 455 0 R (lstlisting.1) 456 0 R (lstlisting.2) 457 0 R ] >>
endobj
98 0 obj
<< /Limits [ (lstlisting.3) (lstnumber.1.13) ] /Names [ (lstlisting.3) 458 0 R (lstnumber.1.1) 459 0 R (lstnumber.1.10) 460 0 R (lstnumber.1.11) 461 0 R (lstnumber.1.12) 462 0 R (lstnumber.1.13) 463 0 R ] >>
endobj
99 0 obj
<< /Limits [ (lstnumber.1.14) (lstnumber.1.5) ] /Names [ (lstnumber.1.14) 464 0 R (lstnumber.1.15) 465 0 R (lstnumber.1.2) 466 0 R (lstnumber.1.3) 467 0 R (lstnumber.1.4) 468 0 R (lstnumber.1.5) 469 0 R ] >>
endobj
100 0 obj
<< /Limits [ (lstnumber.1.6) (lstnumber.2.10) ] /Names [ (lstnumber.1.6) 470 0 R (lstnumber.1.7) 471 0 R (lstnumber.1.8) 472 0 R (lstnumber.1.9) 473 0 R (lstnumber.2.1) 474 0 R (lstnumber.2.10) 475 0 R ] >>
endobj
101 0 obj
<< /Limits [ (lstnumber.2.11) (lstnumber.2.16) ] /Names [ (lstnumber.2.11) 476 0 R (lstnumber.2.12) 477 0 R (lstnumber.2.13) 478 0 R (lstnumber.2.14) 479 0 R (lstnumber.2.15) 480 0 R (lstnumber.2.16) 481 0 R ] >>
endobj
102 0 obj
<< /Limits [ (lstnumber.2.17) (lstnumber.2.21) ] /Names [ (lstnumber.2.17) 482 0 R (lstnumber.2.18) 483 0 R (lstnumber.2.19) 484 0 R (lstnumber.2.2) 485 0 R (lstnumber.2.20) 486 0 R (lstnumber.2.21) 487 0 R ] >>
endobj
103 0 obj
<< /Limits [ (lstnumber.2.22) (lstnumber.2.7) ] /Names [ (lstnumber.2.22) 488 0 R (lstnumber.2.3) 489 0 R (lstnumber.2.4) 490 0 R (lstnumber.2.5) 491 0 R (lstnumber.2.6) 492 0 R (lstnumber.2.7) 493 0 R ] >>
endobj
104 0 obj
<< /Limits [ (lstnumber.2.8) (lstnumber.3.4) ] /Names [ (lstnumber.2.8) 494 0 R (lstnumber.2.9) 495 0 R (lstnumber.3.1) 496 0 R (lstnumber.3.2) 497 0 R (lstnumber.3.3) 498 0 R (lstnumber.3.4) 499 0 R ] >>
endobj
105 0 obj
<< /Limits [ (lstnumber.3.5) (page.11) ] /Names [ (lstnumber.3.5) 500 0 R (lstnumber.3.6) 501 0 R (lstnumber.3.7) 502 0 R (page.1) 503 0 R (page.10) 504 0 R (page.11) 505 0 R ] >>
endobj
106 0 obj
<< /Limits [ (page.12) (page.17) ] /Names [ (page.12) 506 0 R (page.13) 507 0 R (page.14) 508 0 R (page.15) 509 0 R (page.16) 510 0 R (page.17) 511 0 R ] >>
endobj
107 0 obj
<< /Limits [ (page.18) (page.22) ] /Names [ (page.18) 512 0 R (page.19) 513 0 R (page.2) 514 0 R (page.20) 515 0 R (page.21) 516 0 R (page.22) 517 0 R ] >>
endobj
108 0 obj
<< /Limits [ (page.3) (page.8) ] /Names [ (page.3) 518 0 R (page.4) 519 0 R (page.5) 520 0 R (page.6) 521 0 R (page.7) 522 0 R (page.8) 523 0 R ] >>
endobj
109 0 obj
<< /Limits [ (page.9) (section.5) ] /Names [ (page.9) 524 0 R (section.1) 525 0 R (section.2) 526 0 R (section.3) 527 0 R (section.4) 528 0 R (section.5) 529 0 R ] >>
endobj
110 0 obj
<< /Limits [ (subsection.3.1) (subsection.4.2) ] /Names [ (subsection.3.1) 530 0 R (subsection.3.2) 531 0 R (subsection.3.3) 532 0 R (subsection.3.4) 533 0 R (subsection.4.1) 534 0 R (subsection.4.2) 535 0 R ] >>
endobj
111 0 obj
<< /Limits [ (subsection.4.3) (subsubsection.3.3.1) ] /Names [ (subsection.4.3) 536 0 R (subsection.4.4) 537 0 R (subsubsection.3.2.1) 538 0 R (subsubsection.3.2.2) 539 0 R (subsubsection.3.2.3) 540 0 R (subsubsection.3.3.1) 541 0 R ] >>
endobj
112 0 obj
<< /Limits [ (subsubsection.3.3.2) (table.caption.2) ] /Names [ (subsubsection.3.3.2) 542 0 R (subsubsection.3.3.3) 543 0 R (subsubsection.3.3.4) 544 0 R (subsubsection.4.4.1) 545 0 R (subsubsection.4.4.2) 546 0 R (table.caption.2) 547 0 R ] >>
endobj
113 0 obj
<< /Limits [ (table.caption.5) (table.caption.7) ] /Names [ (table.caption.5) 548 0 R (table.caption.7) 549 0 R ] >>
endobj
114 0 obj
<< /BBox [ 0 0 .996 .996 ] /Filter /FlateDecode /FormType 1 /Matrix [ 1 0 0 1 0 0 ] /Resources 550 0 R /Subtype /Form /Type /XObject /Length 8 >>
stream
x┌
endstream
endobj
115 0 obj
<< /pgfprgb [ /Pattern /DeviceRGB ] >>
endobj
116 0 obj
<< >>
endobj
117 0 obj
<< /BaseFont /GFVOWD+XCharter-Bold /Encoding 551 0 R /FirstChar 21 /FontDescriptor 552 0 R /LastChar 122 /Subtype /Type1 /ToUnicode 553 0 R /Type /Font /Widths 554 0 R >>
endobj
118 0 obj
<< /BaseFont /AEKVRN+XCharter-BoldItalic /Encoding 551 0 R /FirstChar 45 /FontDescriptor 555 0 R /LastChar 118 /Subtype /Type1 /ToUnicode 556 0 R /Type /Font /Widths 557 0 R >>
endobj
119 0 obj
<< /BaseFont /GFVOWD+XCharter-Bold /Encoding 558 0 R /FirstChar 65 /FontDescriptor 552 0 R /LastChar 116 /Subtype /Type1 /ToUnicode 559 0 R /Type /Font /Widths 560 0 R >>
endobj
120 0 obj
<< /BaseFont /MHEFEN+XCharter-Roman /Encoding 551 0 R /FirstChar 21 /FontDescriptor 561 0 R /LastChar 122 /Subtype /Type1 /ToUnicode 562 0 R /Type /Font /Widths 563 0 R >>
endobj
121 0 obj
<< /BaseFont /TOHNLJ+XCharter-Italic /Encoding 551 0 R /FirstChar 28 /FontDescriptor 564 0 R /LastChar 122 /Subtype /Type1 /ToUnicode 565 0 R /Type /Font /Widths 566 0 R >>
endobj
122 0 obj
<< /BaseFont /MHEFEN+XCharter-Roman /Encoding 567 0 R /FirstChar 136 /FontDescriptor 561 0 R /LastChar 169 /Subtype /Type1 /ToUnicode 568 0 R /Type /Font /Widths 569 0 R >>
endobj
123 0 obj
<< /BaseFont /GUOWTK+CMSY6 /FirstChar 3 /FontDescriptor 570 0 R /LastChar 3 /Subtype /Type1 /ToUnicode 571 0 R /Type /Font /Widths 572 0 R >>
endobj
124 0 obj
<< /BaseFont /Times-Roman /Encoding /WinAnsiEncoding /Subtype /Type1 /Type /Font >>
endobj
125 0 obj
<< >>
endobj
126 0 obj
<< /BitsPerComponent 8 /ColorSpace /DeviceRGB /Filter /FlateDecode /Height 210 /SMask 573 0 R /Subtype /Image /Type /XObject /Width 1085 /Length 17490 >>
stream
x┌ĒØy\TU ŪDTQ▄┼}═%MK╦Ł┤,5Ęl▒z2Sļ®╠╩L5Ś\DįRqECQAåmdY¢FÜttDŚz×▀┐¾╗8┼├ŃB0sŽ„n¤„ļ╝|Ms╬=„×{┐’{6Ģ
@2M[ŽæÉśD½ ─¼O’µŻV!!50-ODŻ ą$h ą$h ĆČ A[ ĆČ A[ ┤ ┌ Ā-Hą Ā-Hą mAéČ mAéČ hóq$h ĆČ A[ ĆČ A[ ┤ ┌ Ā-Hą Ā-Hą mAéČ mAéČ h┤ @[ÉĀ- @[ÉĀ- ┌ém ┌ém ąh┤ @[ÉĀ- @[ÉĀ- @XŁVCe\čońJnØ+«ī▌Ø¾|hųŹķĮ÷_×ČO?m▀ÕW#.O█¤7ĄņĘ╦]}▌öwŃGŅš$²TŅ7#.OÕ■dŅ┤└įŃĪ- žŹ╔j2T&*ŽĒ╠│#{dH÷╚§iÅĶXē*|Žęö▀ųøė▐°²ÆĪŌ╠▐▄W÷ļ¦ņ╗³╩ZMgä„ą %åÆ¬Ė¾%Ša┘Žäi¤YwĪkCóPG┤ÕqX¼¢ļš║Ō¬žĮ·W÷^×▓.═č>┤ (cu╝▒2>Šl∙╬ņčļ.x∙6>
eĪ-∙ö■ū¬¶▓▀ŠūO▐Ģ3æ?┤ ╚½šP÷ø·\ķW£¬8ģh╦ā¬§GzIĢ·ćK/~øų m r┬p/ĪĶųÖ]9cū^Ós╠ĮČį±/½ķÄµ|Ö «ŗĀą ]«T©Ž¢}”┼(
P[ĻR“{▓║tš«ŗŃßą Ė0>”¶¾Ø┘Ż}┘FĪ"č¢Z╠„╬Wćńīģ @[ Ć8®¬*-╝u*L7Ü,
øČįrźZĮļŌśG.┌īm ¶XŁųé1▀ļ'Łčt"ÄBE½-šīŲXÖ^│■śö┌ ┴|Žp┌e„Ŗ┤┤ź¢ę¬─p▌XtŠ@[Ųh4Ļõł÷>▄ć╠╠L½š*┼Ę1éįø┘l¢\]_├Ź”QW„¹deŃÕjŚą=ĪĀĀ@ßQÉ^»Śųš%Ė╦FlW▓<*į¾@/«<∙ā■ÕĀį┬FĪRčĢVCbyÓ·4o_┤Õ/▐}„]┘▀1°╩+»╝³“╦»▄ć¹░e╦¢ŗ/rŗ¼¼,ā┴ ┬2ŚööÆ»¦¦'Iļ³vŅ▄Ö2;WWū°°xčųFjj¬ā▀░x±b®£·¢-[Š³Čų=e╩ö┌Ų╬²╦²gpp0ūžĄZ-u╦,>ż?╬┴ź^i)))",šņ┘│█╦│šlĖyz¦Ņ9æDĪęę¢┌Ø·JÓ·¶Ņ> hŗĻĮ„▐S°]źO¤>ČPć¹7;;ø┌┼žŹŲvĒ┌ Æuii®äN_RRńö9v’▐]▐Q┘g¤}&Ń÷▐┐[{¤6m┌źKŚĖ&¤ŚŚm!ōb┴Ö7o×8f2Öõ╦ē█Ę╦Žö, Li/¬(TŖ┌RG^ų¦{C^Ā-ĖĮ<@’▐Į¦NØ║~²·▄▄\F©ŪļŖ+$t▓v’▐M£Ż┐┐?┤Efp"├5∙
6p
#Ī▐FhKcčh4Ō,█µ═ø±Ģ¢▀§¦ŗ?Lm/┬(t ÕW═Ę/_┐ōU°AD▐¶G”śę/╣_Ė±{A┼▌éøõVTł¦ōÜ+Ib∙╩uÉhx={÷£1cFFF²X▓sń╬ r╚/Š°óäNąĖqŃ(│suuŹŗŗāČ╚▐bŠ²÷█³³|æÅ(āČ4¢}¹÷ēČl│f═┬®ō[│3gī(╬š¢╠A┴YCāĄCĘe
Įl9h©õōōō%q.BCCēs¶¾¾āČ(ė_ééé─6~┌ę(bccE^┬ģóŁIQXĖ×^XéR:p¬r║°ėéŖōEĘ"XZ┤ÕĪÉņ~(o╬ēó-┐Śč▀ö¼VŻ·JĆó+┌ĻĪ[Ęnkū«Ł¬¬ÆÕsv╬£9Æ8ŃŪÅ¦╠«iė”čččą%3uĻT╩.WhÅ╠¤?_³v,┼%ĶĢ-,tC┬|9UIĒ°]f 3┼¤Õ▐ī▓ZĒyŗ┬B[Ļ”┌¦Ż
V²¦ś°ŅTr=9╝sćh“Šß^┬N▌ ─fh©oo’uļų±{½®©©õX─┐2±v£}¹÷Ģ─EmĪ¦kū«YYYąč#ĪęŖvCLŗšp¾õ¬$w”±Ī_bėÓ¼'/[[ŁvŪ"öČ³9='®ķē¤>¼°Ń
═ē╗kM'©┤įŽ▓e╦°▌õe┬ä éłN¦m%k4WWW╩Ś/_m§_!─mi ’┐ ŠäJ█│gOL╠¦┬.▓Øwś┌.Čt®ŠŖbĮaĄź6ģhG\¼ī 8▐
k┼╬ŗŽ∙'6ģĆ@[@=x{{¾8ź]©qbbQČo▀N£ŃčŻGĪ-Ā~^}§U╩=ÅĀ-
¼% Ź│▒m█6┤&┴1[═1┼¤»Lj┴ne░@M╗│e_RŅ{"m∙S^tD“Æi
JĒ üČĆzĶęź_{ć▒^f∙q╠£9S┤š¹┬/Pfńķķ)Ģy▓ą┴>┘<}hKC8pÓĆõ╩<{÷lL╠Vu
Ģ1+ōY ŗó╦÷¼a9ū„č¤eQiKMJõõÕié9/Ģų╩Ó¼¦Ā!ą@`.▄Øm▄Ėq¶Õ’ųŁøh7'^·ķ¦¤¢╩UmoooÜĒ╬Ī-
až░aR╝Ŗh.!0ČQaŠ¼&░į╦OĘN utóė[l¼VØ³iæŃ╦; Ē
¾|┘
5āČ µµ▓tķR<:k !╬122R*Ś┤Exyy┤h╦▀b0łūŅÓŗw▀}Ēł½špČ¶¾I¼¢¤┌Æ9░ĶųIaÅQ£┌R;jN_qł§mŁĖ*:D;>m§śŗŃāF¬½½)<±"├
dężIö┘Ihä┤Eö¾ż@ßŁhv’ßW¦┤:S“╣×#bų[┌×=\ ļaųw╬Ø║Qś¦mÅ├╦╦╦┴ åkecŲīĪ/∙õ╔ōEX¤ŁZĄó╠Ņ®¦×Æą┼mš+”µm∙█·4hÉt»L╠¦║Nī{r×g%,╔ŁbJ┐Žŗ/±kŗ-Ø*■śuźeÜ┬}a.ąŠ°┬čūŚ_~)H╔333EUōIII─KGEEA[Ć▌µRZZ
mäāJ·ŌÖ={6Zk▒-▓£“S7a2ŹEĒ▓-ļ╔Jkź©Y*┌RėĒó}Jļ8ė╗\Ö%6$ŲĀ-Óč|²§ūÄ³╣PŃ─ęėėEUŹkū«ź╠╬ėėSŻčHĶ2āČł
Ņbįńm®IwĄž(((@bń,ß║±īéĮŁÖā
Ģb|▀%!m∙S^▓×dzŻ│Z
ĪZ¼0mÅÄ^yq-wņž▒¶┼×:u¬©¬č▌▌Ø2╗ß├ćKļ2āČłŪ;[Ī-Źź┤┤Tóō±ļ“ų[oĪ∙0╣<¬b|£ž╠+w9[·╣©”CZ żŌū¬¶oR┌«JqHrHiÉ▄rćŅ┘tėųÜ¶ļČ4ėų┤½█Ė▄×2|Éņ^ōÆĖ_sgĘyM├║]å]dķĆ„W[ŅćcąŻGÅvõnŲ?¶eŅ┘│¦xŅ└zĮŠiė”ö9Jkä┤E┤░õ m®ć?³PŚM▀Š}11¤„V«{ÄY'╦Ā▄øQé GiUµ┘Ō»bKŚ~iŌ“┐ųķŌ■Ź*|Ž«’4ż_▌Æ■ļųŁYC9┘Y!ä┼p&┼¶Ä¦5’±OlOüČĆXČlÖ▌{¹÷mA╩¼šjER{ø7o”╠Nr#─Ā-b&!!┌BŲĆõq┘ŻĒEAu³rFŗ„&8¤-Y,į+>ø¬£/§9W·Ą_R}Į÷i╦├┘iL▀┼¢,]Öž▄WMĻg
│▓▄█źf¬K6īA[└ ÓÕÕe▒XņŠW<¹ņ│¶e■÷█oER{/Š°"evR▄©┌"µČŽoTmyć¢═e¾┌k»ĪĒęXvfÅfČ!╦ Į%RÉāŖ+^v«įŪGĒ▄└ó“ó-u╔╗s·\╔2 $╬_£)ķš┼¹YV®y╗vl┌ĻŌHćŗ !źx¢AvssŻ╠.::┌xõ½»ŠéČ ø«999h;ÄĀĘ─Ņ╠~¢M\ń┤-kqKõ▓;w┼/ņŌ.żllüyū¢║■W“Ģo¹"ē¬KµśVo╠┌ĻąźK╗ć+¤:uJÉ2ŗa¼įŲŹ)│ļ▄╣│GĢC[─īŚŚWaa!┤ģ)ZŁ¢`2ŠććŪ©QŻhÄh╬£9h;v„¾rf╗║7╔╝JĘĘū▐īa:ć³ŗØČ³„.±╦ÜĒHNĶŽėg╗┬XH÷ōīmĆČ@[żł▌ŗ!sĒį┘┘ÖŠ└bžĮeė”Mö┘=∙õōR╝┤Ā-"gõ╚æą”ąLŲ?y“$═
0@T+SI█fĶ¼f▀g¼▓Včłß^┬×ä@[löV┼äh┘nē▓ŁŲ\,ņßóy?┤┌lxyyÖ═v╬,[▓d }ü_yÕ┴+ŁEŗö┘Iqä┤E─ŪŪC[žčĘo_ųYxzzr ÜL&▓ā┌▒cNŃB}s╗XnKµŁÅ╦"ŠžŪ7üŽÆōiŗŹŌ¬ĶĒ║æŠ,ŃĻ£ļwĢšÖĆČ@[ĆŹÕ╦Ś█„ćé,╔█╗woa_„®šj▓ņ:uĻdĘWB[@²šßmyÜ╔°’╝¾qcÖ={6N├I(¾Y╬,ÉėÜ├YŚ▀dę─Ģ,│cĻŖž┤┼FčŹ3Ī┘{^.▌`xF.▐žmüČ Ä¦¤~Z*▒Ŗ
aŚA&×ž"┼5─$¬-/^t³K8Śdo#;łŹŹģČ░ÓŻÅ>"╚eū«]Čö}▓śś▀└FŲl`X═╗²_┘v{½ŃwĶ×fW~A┤┼v^ŌJŚ▓øż¤s}7╗┬ļ« müČ Äsń╬┘„ćéī█░aāĆuš¼Y3╩ņ╬£9mæÉČ<³ł\Ąj┐╦v±╦ß├Ī-╝Żūļ█Čm╦:///AN┴╠Ö3±ą¼ėØėa9ŻÖ═żl03ĻÉwńtł÷ių3┘ģę¢┌÷Æ²Ż%▌tū÷░+yßšŻ> ąģj╦Ū╝é¤|“ēäŅ▒#Fī░’YOLžeÉ)Ą┼ėėė`0@[ż½-u¤Æ▒▒▒v»Ć┴ŪG{B[Ó╚æ#╣ņ▐²?’u/^Lst──³·K╦I▀gf│ø²]PĪ®Ö B▒’ē░┌bŻžĒ├`³ø»┌)8k8├LæµmßĖz§*kØ»eÕ╩ĢŠŠŠ╝╝žd¶ų╬Šģģyx999®šjA*ŖxäžSO=%▌g7┤ÕqMFlØ/ÄŚ┌“ ō±▌▌▌XĮ║║Üņ āāāĪ'Ål;│Ū░█«}ŗvłšj`T“ž▓ÕD5ŖF[lŠāMĘKp├%@ĄµĮJ3hüČ<«Ź°°°ž=Øä111÷²Ī Ń─▓▓▓®ź)S”HŌż@[D½-Ą„üźKŚŖńŁģā▐m®╦▒cŪ£££XńŌĒĒ²Y░╗▀╝▒╠ś1Æ“p²će?ŪpĢ▌LV╗IŲ¹¼HjNģŖD[l*O29eY├┘▌Ģf.ąĪ┤Õ┐7Ŗ°xQ╔ŗ▌ģß×č¶źjõ”MøRfg„£#hŗ╚Ąź6╬=z┤N¢āŻ=Ī-uĪÖī┐gŽøī│││░KŻł
²Ē©UI-ž┼l[││he┼Uč!┘#ēBEź-¬ÜæWńX█«Ųį\|Ī-ą“hßÖg×C²█ĮbĢ Ń─·÷ĒK¤’w▀}GÖØhćB[°E¹“8°¢┌RŗFŻ!śī’ĒĒ²╚:¦\,}ų¼Y░7¹%6eĖ9K ╬┬}a©÷┐─&BEĪbė[Øņ`░°[H÷S0hŗl┤┼ųRg§█3╗▀Ė
R~·w}ļųŁŻ╠Älé-┤EXmQ▌_ģ°╣ń×dyyy9▓°┤ź¢łłé\µ╬Ø+°ļÄAāab>ć·ńĢlØģA?KµŹĆõ¢4čft±Ū1źKbŗ┐<]Č$÷~Ŗ)²³l╔Æ@M█š)Ł«¼N¹5Xc┌)ss╔bh.z╦h┤ģAČn| ╗ū┌=~³8}i7m┌D£cŗ-(│KHHĆČ(D[lÅ╦1cŲXĆō'OB[¦ ■╣į│Ś%ÕŻDß¾k&Óńīe¬±▐Žr┐ōeö_ó╗2ćdÅ8m°,ūrįjĄįS°§ķ▌Ļ,ĮźZĢņ■MJĄqeQĄk■*╬ĮČXMĄ░4ŚCąhŗ═eį©QR Z8&N£Hz3”┘ų"ķ5─Ā-R4Śæ#GB[$""┬┘┘Öu.OŲĻ\LØ:╬"Ī~¢éĻĶ-½Ö,╦š¬h├¦ĘŁĘXµ║┌R7∙'║”Č;o¶/║%õŖ4∙U¦|x▐╣&ŃĻ6åŽ»½X╠═üČ@[ĻGÉnŗZ┘ÉØ8┬T▌¤O9NlĒ┌ĄöGG_¤ą┴Ą┼Ź;VÉ¼YO┌bāf2■█Ą<╠ŪLv╬╔╔Qż│Tä3v¢ŁÖ|:„Uń╦³²┘f█æ=2 fdcWf~£Čį§Ś░£ńJ½ōCe┐+B¹½]│M;žXo▐müČąŃļļ+Tų]║t1ÖL*6eĶ“Å³ā“ąŌŌŌĀ-
įš²y.MÜ4$k╗ĆČž*ĪMø6¼siš¬š▀Ņ▒Ivįsµ╠QÜ│h═{S█▒u-¤╬b▒ZB┘t▓äf?]tļĖ}E²[m∙K^Üåķ×¬ń%┐ĻĖ»ÜŽu£ŗe]█┼«└gJ>ģČ@[Ķ▒{░¢Ńž=Š]ÉqböC0B┌B®...¶∙·°°@[ņf▀Š}╣╝∙µøó:Jøś¤k9╩zŗō¹¾Yx[Nw=öģd∙¬øVD9rĻ©-Ąč~╝qź WZ■u■═ģ;)ņ^×kćA[Ā-─©šj╩ ∙üx╔Ņ┐:t(qi¹§ļGs█ČmÕq}“╔'ą%kŪĖqŃĶ3}Ōē'Ā-vC3 ąĪC
∙Ą²ļ_d¬RzskgY«V▌Č¢¾UÓėv.HµĘä~ēMó
¤XŁF╦ų(m®ŹViŁż?’7ó°],Ņ╗ī■ĘŁĘÜKµPh┤ģ???A“udų»┐■ÜŠ└ēēŚ“╠Ö3)*>>┌ópmß=nnn─Ö┌=Į┌r¶ĶQé╔°=z¶h`UWTTÉ¹õ╔ōĢÓ,»Od╝Ģ|éŖ»„▄uó.[╔╗│ņ╚źĘ│vÉ┌rX┌AīÕ▐ŖŌwfPH÷HvĘM½š╠]KąhqąŌĻĻJ¤’Ó┴ā)3}ü7o▐,│ćŃ╚æ#Õ1Ķ┌Ō +W«żŽįŠ§Ī-4│Ó▀xŃŹå¤æ!Cåą{¾µ═§zĮņĄÕ╝qļ└,█ŲW³░=k’J×3ÄŪ¢n¤Č▄dš,²WV▐Hn═oĢąOcTTŗEÅ▐h=éLrwIńł§žŪ°±ŃYgĪėķ(ÅH#─Ā-╝─-[ČäČł_[Ė├o▌║5ļ\ZĄjš©Kö▓¹{ń╬Ø*╣¾ļžåé█2∙yzÜŁµĒ┘<;g
╣ūy^ń┴nmßę
rs9U╝žG═¾z╚EĘN│(¬▐¬ &┼┌mĪ'>>×~A!Ąģ~l[¾µ═Y/ā(Łh┌"mQ čßb▀z
ū¢={÷õę»_┐F²Š┴` ½%L╠ńpŽźI╠ČhyéŚ
¼▓Vģ▌ŽŌ¤į¼ĶŲ1▐ļėmßęŲ¶>wŅśh╬{t±┐xw¢åĢvuJL╔ćČ²ÓvWWū┐]]STčŗĻ■
L┐▀├├āņXd3B┌┬ū3łĖ├e└ĆąÜJk,;v4zŪ╩Ņ’┤┤4Ģ▄Q_±¾eÆ┼Ś∙;^<.īß╗¤%${d×ēI¦Ćā┌bøĪ▀ž═bŗčj\sĪ#’¦;┌░ł┼=ō¹╬Ń¾░ 2┤E@¢/_N¤®#ŗē®Xī╚n”O¤╬Ņ╦Ē3c7d¹─A[$Ī-cŲīĪ╠╬ėėėÄń®ÆĄź┤┤┤U½V¼si╚v-s¶ĶQ▓zh°╝ķbĄūźueÆj┌9žłj·Y°▐£e„┼x\ŖÖwmßęŚ_fw║═„k5Øx?ūkS;Öį*w²³X$[güČHE[┘x╬ŅŁ[l¼X▒éĖ└²¹„g5Pŗ#]]ąYjwh┌┤)eÄv©║ÆĄģµUā}╗:R×777%L╠Å7·3Ŗ╩R╠-│_P├Ę│Ldz²ó-+ōÜg^e2»╩Pżķ╠¹Y║ąčT╔õ)╝Ķ};┤E*┌ó"▀£]ÕpoÕĆĻZ}¾ņ┘│╔ÄBN#─Ā-

---

