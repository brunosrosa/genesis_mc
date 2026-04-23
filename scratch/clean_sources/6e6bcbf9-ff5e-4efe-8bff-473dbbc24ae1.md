# Deep Kernel Fusion for Transformers - arXiv
Source URL: https://arxiv.org/html/2602.11808v1

Source Type: web_page

Source ID: 6e6bcbf9-ff5e-4efe-8bff-473dbbc24ae1


Deep Kernel Fusion for Transformers
Abstract
Agentic LLM inference with long contexts is increasingly limited by memory bandwidth rather than compute. In this setting, SwiGLU MLP blocks, whose large weights exceed cache capacity, become a major yet under-optimized bottleneck. We propose DeepFusionKernel, a deeply fused kernel that cuts HBM traffic and boosts cache reuse, delivering up to 13.2% speedup on H100 and 9.7% on A100 over SGLang. Integrated with SGLang and paired with a kernel scheduler, DeepFusionKernel ensures consistent accelerations over generation lengths, while remaining adaptable to diverse models, inference configurations, and hardware platforms.
Deep Kernel Fusion for Transformers
Zixi Zhang Imperial College London London, UK b.zhang25@imperial.ac.uk Zhiwen Mo Imperial College London London, UK zhiwen.mo25@imperial.ac.uk
Yiren Zhao Imperial College London London, UK a.zhao@imperial.ac.uk Robert Mullins University of Cambridge Cambridge, UK robert.mullins@cl.cam.ac.uk
1 Introduction
The Transformer architecture (Vaswani et al., 2023) underpins modern large language models (LLMs) (Brown et al., 2020; OpenAI et al., 2024; DeepSeek-AI et al., 2025a), and as these models are deployed in agentic, real-world workloads, inference efficiency becomes a first-order concern (Aminabadi et al., 2022; Zhou et al., 2024; Yao et al., 2023). Agentic workloads require the model to handle very long contexts, maintain many persistent state items, and often produce long-form outputs (for example, large codebases). As a result, they process on the order of more tokens per inference than typical chatbot-style workloads (Wu et al., 2025). In this regime, caching and fast access to KV values shift the primary bottleneck from raw compute to memory capacity and memory bandwidth. That shift in turn changes the optimization landscape: permissible batch sizes shrink, GEMM inputs become “fat,” Tensor Core utilization drops, and available compute is severely underused.
Most recent GPU optimizations focus on attention (Shah et al., 2024; Kwon et al., 2023; Zheng et al., 2024; Ye et al., 2025), but in autoregressive decoding, the SwiGLU MLP blocks (Shazeer, 2020; Grattafiori et al., 2024; Bai et al., 2023; DeepSeek-AI et al., 2025b) dominate parameter count and drive memory-bandwidth pressure. Although modern accelerators offer high HBM bandwidth (NVIDIA Corporation, 2021, 2024, 2025a), memory scaling has not kept pace with compute, so memory-bound kernels remain the limiting factor for throughput on many real deployments.
To close this gap, we propose aggressive deep kernel fusion to reduce memory traffic and improve utilization in FFN blocks in the attention mechanism. We implement a highly optimized fused operator, the DeepFusionKernel, which combines the separate GEMMs and pointwise kernels used in common four-kernel and two-kernel SwiGLU implementations (e.g., PyTorch (Paszke et al., 2019), SGLang (Zheng et al., 2024), and vLLM (Kwon et al., 2023)) into a single fused kernel that minimizes intermediate memory reads/writes and exposes more efficient work for Tensor Cores. Integrated with a lightweight, profile-driven kernel scheduler and deployed inside SGLang, DeepFusionKernel yields consistent, deployable speedups in bandwidth-bound agentic scenarios: up to 9.7% on A100 and 13.2% on H100 clusters compared to the SOTA SGLang implementation.
2 Background
Modern Transformer-based LLMs replace the original two-layer ReLU design in the MLP with the gating-integrated SwiGLU variant (Shazeer, 2020):
|
|
Here and , with being 3.5 to 4 times the . These large matrices dominate the model’s parameter count and memory footprint.
3 Method
Kernel fusion (Aminabadi et al., 2022; NVIDIA Corporation, 2025b; NVIDIA, 2018) reduces end-to-end latency and memory traffic by combining sequences of operations (GEMMs, pointwise nonlinearities, and simple elementwise math) into a single GPU kernel, thereby eliminating intermediate reads and writes–the dominant cost in bandwidth-bound LLM decoding (Kwon et al., 2023; Zheng et al., 2024). Fusion is most effective for elementwise and tile-local computations; true reductions (e.g., Softmax) introduce long-range dependencies that limit cross-SM streaming and therefore are not good fusion targets. Guided by this observation, we split the Transformer MLP into two stages:
|
|
and focus fusion effort on the first stage, which contains the GEMMs and pointwise gating that dominate memory traffic during autoregressive decoding.
As illustrated in Figure˜1, DeepFusionKernel fuses the separate GEMMs and pointwise kernels (four launches in a naive PyTorch implementation, two in SGLang/vLLM) into a single deeply fused operator. The fused kernel streams intermediate values through the computation, avoiding materialization of large temporaries and drastically cutting reads/writes to HBM. To realize this in practice, we systematically explore loop ordering and tiling strategies that maximize on-chip reuse and arithmetic intensity. Concretely, we observe the following trade-offs in our experiments:
-
•
Row-major tiling: improves locality for the input activation , reducing repeated loads of input rows and benefiting scenarios with larger batch sizes or when activations dominate memory traffic.
-
•
Column-major tiling: better reuses weight tiles, which is preferable when model weights dominate memory footprint (a common case in agentic decoding with small batch sizes and large models).
We integrate these tiling and loop-ordering choices into the fused kernel implementation and tune tile sizes to balance register usage, shared-memory occupancy, and Tensor Core utilization. Because the best kernel configuration depends on model shapes, batch size, GPU microarchitecture, and distributed interconnects, we accompany DeepFusionKernel with a lightweight profiler-driven scheduler. At deployment time, the scheduler quickly benchmarks the set of candidate kernels prior to the inference on the target hardware and selects the highest-throughput option for the given model and workload, ensuring robust performance across architectures and real-world inference conditions.
4 Experiments
We integrate DeepFusionKernel into a complete inference pipeline and measure end-to-end decoding throughput in a realistic framework to isolate practical benefits and deployment considerations. To minimize kernel invocation and CPU overhead while maintaining a consistent evaluation environment, we run all end-to-end experiments inside the SGLang inference framework, with FlashInfer and CUDA Graphs enabled. We compare against three baselines: PyTorch (naive distributed), SGLang (default kernels), and vLLM. A lightweight kernel scheduler selects the best fused-kernel variant for each hardware/configuration before measurement.
4.1 Experiment Setup
Kernel performance depends on memory bandwidth, compute capability, model shape, and interconnect topology. To stress memory behavior, we run tensor-parallel inference across four NVIDIA A100 or H100 80 GB SXM GPUs (TP=4), which exposes both HBM access and inter-GPU communication effects. We first evaluate LLaMA 3.1 70B in FP16 with fixed prompt length 1 and target output lengths of 1024 tokens; batch sizes vary from 1 to 64. Each configuration is measured four times; we report mean throughput and standard deviation. For long-generation experiments (Section˜4.3), output length is swept to probe KV-cache and attention effects.
4.2 Full-Model Throughput
| Batch size | A100 GPU cluster | H100 GPU cluster | ||||||
| Torch Throughput | SGLang Throughput | DeepFusionKernel | Torch Throughput | SGLang Throughput | DeepFusionKernel | |||
| Throughput | Speedup | Throughput | Speedup | |||||
| 1 | +5.7% | +3.4% | ||||||
| 2 | +7.7% | +13.2% | ||||||
| 4 | +4.9% | +3.6% | ||||||
| 8 | +6.1% | +4.2% | ||||||
| 16 | +9.7% | +4.2% | ||||||
| 32 | +3.9% | +5.5% | ||||||
| 64 | +1.3% | +3.4% |
Table˜1 compares end-to-end decoding throughput across frameworks and GPUs. Consistent with prior work, SGLang substantially outperforms a naive distributed PyTorch baseline. Integrating DeepFusionKernel into SGLang yields additional gains–up to 9.7% on A100 and 13.2% on H100 for typical batch-size-limited decoding workloads–by reducing memory traffic in the SwiGLU MLPs.
The observed behavior follows expected bottleneck shifts: On A100, the benefit is largest at small batch sizes, where the workload is strongly memory-bandwidth-bound, and reduced reads/writes directly raise throughput. On H100, the kernel maintains an advantage across a wider batch-size range; H100’s much higher compute throughput (1979 TFLOPs/s versus 312 TFLOPs/s on A100) means memory bandwidth continues to be a salient limiter, so memory-traffic reductions from fusion still translate to measurable speedups, though with diminishing marginal returns at very large batches.
Throughput variance grows with batch size across all setups, driven primarily by jitter in inter-GPU communication. Because DeepFusionKernel reuses SGLang’s existing all-reduce and collective primitives, fusion does not change communication patterns and thus inherits this variance.
4.3 Agentic Long-Generation Evaluation
Agentic workloads generate very long outputs and accumulate large KV caches, increasing off-chip memory pressure and per-token latency. To evaluate this regime, we benchmark LLaMA 3.1 70B for output lengths from 1024 up to 16384 tokens, using three representative batch sizes: (single-query), (light concurrent inference), and (moderate concurrency). Tests are run in FP16 and under TP=4 on both A100 and H100 clusters. For consistency, we measure only the decoding stage.
Results in Table˜2 show DeepFusionKernel consistently improves throughput versus SGLang and vLLM across generation lengths and concurrency levels. Speedup variability is mostly explained by communication jitter and occasional system-level noise at high concurrency; nevertheless, the MLP remains a significant fraction of per-token latency even for long sequences, so memory-traffic reduction from fusion yields persistent gains.
| GPU | A100 | H100 | |||||
| Batch size | Output len | 1024 | 4096 | 16384 | 1024 | 4096 | 16384 |
| 1 | SGLang | ||||||
| vLLM | |||||||
| DeepFusionKernel | |||||||
| 4 | SGLang | ||||||
| vLLM | |||||||
| DeepFusionKernel | |||||||
| 16 | SGLang | ||||||
| vLLM | |||||||
| DeepFusionKernel |
4.4 Discussion
Our experimental results demonstrate that DeepFusionKernel with a profiler-driven scheduler consistently improves decoding throughput across realistic, bandwidth-bound inference scenarios. Key observations are:
-
•
Robust gains in memory-bound regimes. Fusion produces the largest improvements when workloads are memory-bandwidth-limited (small batches, large models, long contexts). Even on H100, where compute is abundant, reducing memory traffic yields meaningful speedups.
-
•
Stable behavior for long-context generation. As sequence length and KV cache grow, attention cost increases, but the MLP still contributes substantially to per-token cost; fusion therefore remains beneficial across long-generation workloads.
-
•
Low deployment overhead. Kernel selection is performed with a short pre-inference profiling step; after selection, we use CUDA Graphs for capture, so there is no recurring inference-time dispatch overhead from the scheduler.
Overall, DeepFusionKernel provides a practical, deployable improvement to existing inference stacks: it reduces memory traffic in the critical SwiGLU path, integrates with current frameworks, and produces repeatable throughput gains across hardware and workload regimes.
5 Related work
Existing frameworks like Apex (NVIDIA, 2018), TensorRT-LLM (NVIDIA Corporation, 2025b), and DeepSpeed-MII (Microsoft, 2022) use shallow fusion (e.g., GEMM+activation), leaving significant overheads for larger MLPs. Automatic compile-time fusion approaches exist: Welder (Shi et al., 2023) fuses based on tile-graph cost models but is limited to linear chains; TVM (Chen et al., 2018) applies pattern matching and heuristics but its template-driven method mostly handles small trees; Blockbuster (Dekel, 2025) uses algebraic rules and demonstrated a SwiGLU prototype, but remains a standalone compiler study and lacks runtime feedback and hardware-aware tuning. In contrast, DeepFusionKernel deeply fuses the full SwiGLU into a single kernel. Paired with the kernel scheduler, it adapts fusion depth to workload and GPU, delivering consistent, branch-free speedups.
6 Conclusion
We present DeepFusionKernel, an aggressively fused CUDA operator that eliminates intermediate buffers in the SwiGLU MLP and rebalances the trade-off between memory traffic and on-chip compute. When integrated into SGLang and driven by a lightweight profiler-based scheduler, the fused kernel produces consistent, deployable throughput improvements–up to 9.7% on A100 and 13.2% on H100–across batch sizes and long-generation agentic workloads. By targeting the memory-bandwidth bottleneck that dominates autoregressive decoding, DeepFusionKernel lets modern GPUs better realize their available compute, making it a practical optimization for real-world LLM inference pipelines.
Limitations
Due to limited computing resources, we do not exhaustively evaluate different GPU cluster interconnects, though our tests indicate that any resulting performance degradation is minimal for the workloads studied. Similarly, we do not quantify performance variation from inter-GPU communication, but its impact is mitigated by using the same inter-device reduction strategy as the baseline framework.
References
- DeepSpeed inference: enabling efficient inference of transformer models at unprecedented scale. External Links: 2207.00032, Link Cited by: §1, §3.
- Qwen technical report. External Links: 2309.16609, Link Cited by: §1.
- Prompting is programming: a query language for large language models. Proceedings of the ACM on Programming Languages 7 (PLDI), pp. 1946–1969. External Links: ISSN 2475-1421, Link, Document Cited by: §A.2.
- Language models are few-shot learners. External Links: 2005.14165, Link Cited by: §1.
- TVM: an automated end-to-end optimizing compiler for deep learning. External Links: 1802.04799, Link Cited by: §5.
- DeepSeek-R1: incentivizing reasoning capability in llms via reinforcement learning. External Links: 2501.12948, Link Cited by: §1.
- DeepSeek-V3 technical report. External Links: 2412.19437, Link Cited by: §1.
- Blockbuster, part 1: block-level AI operator fusion. External Links: 2505.07829, Link Cited by: §5.
- The Llama 3 herd of models. External Links: 2407.21783, Link Cited by: 1st item, §1.
- FlashMLA: efficient MLA decoding kernels. GitHub. Note: https://github.com/deepseek-ai/FlashMLA Cited by: §A.2.
- Efficient memory management for large language model serving with PagedAttention. External Links: 2309.06180, Link Cited by: 4th item, §A.2, §1, §1, §3.
- DeepSpeed Model Implementations for Inference (MII). Note: https://github.com/deepspeedai/DeepSpeed-MIIAccessed: 2025-05-26 Cited by: §5.
- NVIDIA A100 Tensor Core GPU datasheet. Note: https://www.nvidia.com/content/dam/en-zz/Solutions/Data-Center/a100/pdf/nvidia-a100-datasheet-us-nvidia-1758950-r4-web.pdfAccessed: 2025-05-26 Cited by: §A.2, §1.
- NVIDIA H100 Tensor Core GPU datasheet. Note: https://resources.nvidia.com/en-us-gpu-resources/h100-datasheet-24306Accessed: 2025-05-26 Cited by: §1.
- NVIDIA DGX B200 datasheet. Note: https://resources.nvidia.com/en-us-dgx-systems/dgx-b200-datasheetAccessed: 2025-05-26 Cited by: §1.
- NVIDIA TensorRT-LLM documentation. Note: https://nvidia.github.io/TensorRT-LLM/Accessed: 2025-05-26 Cited by: §3, §5.
- Apex. GitHub. Note: https://github.com/nvidia/apex Cited by: §3, §5.
- OpenAI o1 system card. External Links: 2412.16720, Link Cited by: §1.
- PyTorch: an imperative style, high-performance deep learning library. External Links: 1912.01703, Link Cited by: 2nd item, §1.
- FlashAttention-3: fast and accurate attention with asynchrony and low-precision. External Links: 2407.08608, Link Cited by: §A.2, §1.
- GLU variants improve transformer. External Links: 2002.05202, Link Cited by: §1, §2.
- Welder: scheduling deep learning memory access via tile-graph. In 17th USENIX Symposium on Operating Systems Design and Implementation (OSDI 23), Boston, MA, pp. 701–718. External Links: ISBN 978-1-939133-34-2, Link Cited by: §5.
- Triton: an intermediate language and compiler for tiled neural network computations. In Proceedings of the 3rd ACM SIGPLAN International Workshop on Machine Learning and Programming Languages, pp. 10–19. Cited by: §A.2.
- Attention is all you need. External Links: 1706.03762 Cited by: §1.
- Combating the memory walls: optimization pathways for long-context agentic llm inference. External Links: 2509.09505, Link Cited by: §1.
- ReAct: synergizing reasoning and acting in language models. External Links: 2210.03629, Link Cited by: §1.
- FlashInfer: efficient and customizable attention engine for LLM inference serving. External Links: 2501.01005, Link Cited by: §A.2, §A.2, §1.
- SGLang: efficient execution of structured language model programs. External Links: 2312.07104, Link Cited by: 3rd item, §A.2, §1, §1, §3.
- A survey on efficient inference for large language models. External Links: 2404.14294, Link Cited by: §1.
Appendix A Technical Appendices and Supplementary Material
A.1 Distributed Inference
In distributed inference, computation is spread across multiple GPUs, either within a single server or across multiple server nodes. This setup introduces communication overheads, primarily due to all-reduce (for aggregating results) and all-gather (for collecting distributed outputs) operations. The frequency and cost of these operations are influenced by the kernel fusion strategy employed. The magnitude of the overheads generally scales with the volume of data transferred and can vary logarithmically, linearly, or quadratically with the number of devices or nodes, depending on the interconnect architecture.
A common strategy for distributed computation in large language models is tensor parallelism (TP). For a matrix multiplication , where , , and , TP involves partitioning the weight matrix across devices, either row-wise (producing tall matrices) or column-wise (producing wide matrices). Each GPU computes a slice of the output , after which an all-gather operation is required to assemble the full result.
However, in the case of compound matrix operations like SwiGLU MLP–which comprises two consecutive matrix multiplications with an intermediate nonlinearity–we can reduce communication overhead. Specifically, as demonstrated in Figure˜2, we partition and column-wise and row-wise. This configuration allows intermediate computations to remain local to each device, requiring only a single all-reduce at the end to aggregate the final output. Consequently, we reduce the number of collective operations to just one all-reduce per SwiGLU MLP block.
A.2 Full-Model Performance Evaluation Experiment Setup
SGLang (Zheng et al., 2024) is a high-performance inference framework for large language models (LLMs), demonstrating up to higher throughput than other state-of-the-art systems, including vLLM (Kwon et al., 2023) and LMQL (Beurer-Kellner et al., 2023).
SGLang achieves this through a suite of custom optimizations tailored for efficient single- and multi-GPU inference, including:
-
•
RadixAttention for efficient key-value (KV) cache management;
-
•
Grouped GEMMs for kernel fusion across parallel matrix multiplications;
-
•
Fused kernels for common patterns of elementwise operations;
-
•
Optimized scheduling for consecutive matrix multiplications under tensor parallelism (TP);
-
•
Custom cross-device all-reduce kernels;
-
•
TP-adapted model head, combining a linear projection and a Softmax for next-token prediction.
SGLang supports several SOTA attention backends, including Triton (Tillet et al., 2019), FlashAttention-3 (Shah et al., 2024), FlashMLA (Jiashi Li, 2025), and FlashInfer (Ye et al., 2025). Additionally, SGLang captures CUDA Graphs during a warm-up phase, allowing inference to proceed without per-iteration PyTorch API or kernel launch overheads.
As kernel performance depends on both memory bandwidth and compute capacity, we evaluate DeepFusionKernel inference across multiple SOTA GPUs.
Due to the large model size, we use tensor parallelism (TP) to partition weight matrices across GPUs. TP does not increase the number of FLOPs or alter arithmetic intensity, assuming the fused computation strategy described in Section˜A.1 is followed.
However, TP introduces extra HBM access and communication latency during the all-reduce step that follows the TP-adapted projection. The impact of this latency grows with the amount of data to be synchronized and is sensitive to both the number of GPUs and their interconnect configuration. For example, in an NVIDIA A100 SXM cluster, each pair of GPUs is connected via NVIDIA NVLink (600GB/s), while the connections to the host system and between more GPUs employ the standard PCIe Gen4 links (64 GB/s) (NVIDIA Corporation, 2021). Inter-device communication is also susceptible to packet drops, which introduces latency variability.
For this evaluation, we use SGLang with the FlashInfer attention backend (Ye et al., 2025) and CUDA Graphs enabled so that once profiling identifies the best kernel, SGLang captures a complete CUDA Graph, eliminating branching during inference. We compare three setups:
-
•
Torch distributed inference
-
•
SGLang with default kernels
-
•
SGLang with DeepFusionKernel
We run Llama 3.1 70B in FP16 with TP across four NVIDIA A100 80GB SXM GPUs, all on a single node. To focus on decoding performance, we use an input sequence length of 1 and an output sequence length of 1024. Batch sizes range from to . Each experiment is repeated four times, and we report the mean and standard deviation of decoding throughput.
A.3 Licenses of Models and Frameworks
-
•
Llama 3.1 70B (Grattafiori et al., 2024): Llama 3.1 Community License Agreement. Link: https://huggingface.co/meta-llama/Llama-3.1-70B and https://huggingface.co/meta-llama/Llama-3.1-405B.
-
•
PyTorch 2.6.0 (Paszke et al., 2019): Please refer to the official GitHub page: https://github.com/pytorch/pytorch/tree/v2.6.0.
-
•
SGLang v0.4.6.post4 (Zheng et al., 2024): Apache-2.0 license. GitHub page link: https://github.com/sgl-project/sglang/tree/v0.4.6.post4.
-
•
vLLM v0.10.1.1 (Kwon et al., 2023): Apache-2.0 license. Github page link: https://github.com/vllm-project/vllm/tree/v0.10.1.1.