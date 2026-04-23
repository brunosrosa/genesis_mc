# 🧠 SODA CANONICAL KNOWLEDGE BASE
> Gerado pelo @soda-knowledge-curator.
> Fontes Originais: 249 | Fontes Puras: 247 | Temas Identificados: 24



## 🧩 Eixo Temático 13

# Accelerating Vision Diffusion Transformers with Skip Branches - arXiv
Source URL: https://arxiv.org/html/2411.17616v1

Source Type: web_page

Source ID: 533723c4-9d16-43f1-87fc-fc753e18a9f3


Accelerating Vision Diffusion Transformers with Skip Branches
Abstract
Diffusion Transformers (DiT), an emerging image and video generation model architecture, has demonstrated great potential because of its high generation quality and scalability properties. Despite the impressive performance, its practical deployment is constrained by computational complexity and redundancy in the sequential denoising process. While feature caching across timesteps has proven effective in accelerating diffusion models, its application to DiT is limited by fundamental architectural differences from U-Net-based approaches. Through empirical analysis of DiT feature dynamics, we identify that significant feature variation between DiT blocks presents a key challenge for feature reusability. To address this, we convert standard DiT into Skip-DiT with skip branches to enhance feature smoothness. Further, we introduce Skip-Cache which utilizes the skip branches to cache DiT features across timesteps at the inference time. We validated effectiveness of our proposal on different DiT backbones for video and image generation, showcasing skip branches to help preserve generation quality and achieve higher speedup. Experimental results indicate that Skip-DiT achieves a speedup almost for free and a speedup with only a minor reduction in quantitative metrics. Code is available at https://github.com/OpenSparseLLMs/Skip-DiT.git.
1 Introduction
Diffusion models [9, 3, 24, 46] have emerged as the de-facto solution for visual generation, owing to their high fidelity outputs and ability to incorporate various conditioning signals, particularly natural language. Classical diffusion models, which adopt U-Net [27] as their denoising backbone, have dominated image and video generation applications. More recently, Diffusion Transformers (DiT) [4, 23] have introduced an alternative architecture that replaces traditional sequential convolutional networks with Vision Transformers, offering enhanced scalability potential. While initially designed for image generation, DiT has demonstrated remarkable effectiveness when extended to video generation tasks [19, 16, 25]. However, despite these advances, significant challenges remain in scaling diffusion models efficiently, particularly for applications involving large numbers of input tokens such as video generation. This scaling challenge is especially pronounced in DiT architectures, where the attention mechanism’s computational complexity grows quadratically with input size. The magnitude of this challenge is illustrated by Sora [20], a state-of-the-art video generation model that remains unavailable to the public.
Numerous approaches have been proposed to enhance the efficiency of diffusion models, including reduced sampling techniques [33], distillation methods [41, 29], and quantization strategies [5]. Caching mechanisms, which reuse noise latents across timesteps, have emerged as a particularly promising direction as they do not require extensive model retraining [18, 15, 38, 47, 6]. However, many existing caching approaches [18, 38] are specifically tailored to U-Net architectures, leveraging their unique structural properties—particularly the skip connections between paired downsampling and upsampling blocks that enable high-level feature reuse while selectively updating low-level features. While some recent studies [47, 6] have attempted to adapt caching mechanisms for DiT acceleration, they have not achieved the same level of efficiency gains and performance preservation as their U-Net counterparts.
To understand the key challenges of feature caching in DiT, we analyze the feature dynamics during the denoising process. Drawing inspiration from loss landscape visualization techniques [13, 11], we examine feature changes using the early and late timesteps in denoising as a case study. For effective caching, features should exhibit minimal variation between timesteps, allowing us to reuse features from previous steps and bypass computation in subsequent Transformer blocks. We term this property ”feature smoothness”, which manifests as flatness in the landscape visualization. However, as illustrated in Figure 2, we observe that vanilla DiT (w/o skip branches) exhibits high feature variance across timesteps, contrary to the desired characteristics for effective caching. Thus, we ask:
Drawing from the findings in [13], where residual connections are shown capable of mitigating sharp loss landscapes, in this study, ❶ we first conduct preliminary experiments adding skip branches to pre-trained DiT model from shallow to deep blocks, named Skip-DiT . which achieves significantly improved feature smoothness with minimal continuous pre-training, as demonstrated in Figure 2 (w/ skip branches). ❷ We then leverage these skip branches during inference to implement an efficient caching mechanism, Skip-Cache , where only the first DiT block’s output needs to be computed for subsequent timesteps while deep block outputs are cached and reused. ❸ To evaluate our proposal, we conduct extensive experiments across multiple DiT backbones, covering image and video generation, class-conditioned and text-conditioned generation. We demonstrate that Skip-DiT consistently outperforms both dense baselines and existing caching mechanisms in both qualitative and quantitative evaluations. Our contributions are three-fold:
-
•
We identify feature smoothness as a critical factor limiting the effectiveness of cross-timestep feature caching in DiT, which helps better understand caching efficiency.
-
•
We build Skip-DiT , a skip-branch augmented DiT architecture that enhances feature smoothness, and Skip-Cache that efficiently leveraging skip branches for feature caching across timesteps. Skip-Cache facilitates accelerated inference via caching while maintaining visual generation performance.
-
•
Extensive empirical evaluations demonstrate that Skip-Cache achieves substantial acceleration: up to speedup is achieved almost for free and a speedup with only a minor reduction in quantitative metrics.
2 Related Works
Transformer-based Diffusion Models
Diffusion model has become the dominating architecture for image and video generation, whose main idea is iterative generate high-fidelity images or video frames from noise [26]. Early diffusion models mainly employ U-Net as their denoising backbone [24, 3]. However, U-Net architectures struggle to model long-range dependencies due to the local nature of convolutions. Researchers proposing diffusion transformer model (DiT) for image generation [4, 2, 23]. Recent years have witnessed a significant growth in studies of video DiT. Proprietary DiT such as Sora [20] and Movie-Gen [25] show impressive generation quality, also evidenced by open-sourced implementation [48, 12]. Latte decomposes the spatial and temporal dimensions into four efficient variants for handling video tokens, allowing effective modeling of the substantial number of tokens extracted from videos in the latent space [19]. CogvideoX adds a 3D VAE combined with an expert transformer using adaptive LayerNorm, which enables the generation of longer, high-resolution videos [40]. However, as the number of tokens grows exponentially with video length and spatial resolution, the computational complexity of DiT especially the self-attention mechanism remains a significant bottleneck for video generation.
Diffusion Acceleration with Feature Caching
Since Diffusion model involves iterative denoising, caching features across time-steps, model layers, and modules has been found an effective way to save inference computation costs. For U-Net Diffusion, DeepCache [18] and FRDiff [32] exploit temporal redundancy by reusing features across adjacent denoising steps. While other works take a more structured approach by analyzing and caching specific architectural components–Faster Diffusion [15] specifically targets encoder feature reuse while enabling parallel decoder computation, and Block Caching [38] introduces automated caching schedules for different network blocks based on their temporal stability patterns. Recently, cache-based acceleration has also been applied to DiT. PAB [47] introduces a pyramid broadcasting strategy for attention outputs. -DiT [6] proposes adaptive caching of different DiT blocks based on their roles in a generation–rear blocks during early sampling for details and front blocks during later stages for outlines. T-Gate [45] identifies a natural two-stage inference process, enabling the caching and reuse of text-oriented semantic features after the initial semantics-planning stage. While these caching techniques have shown promise, they are primarily limited to inference-time optimization, and there remains significant potential for improving their acceleration factors.
3 Methodology
3.1 Preliminaries
Diffusion model
The concept of diffusion models mirrors particle dispersion physics, where particles spread out with random motion. It involves forward and backward diffusion. The forward phase adds noise to data across timesteps. Starting from data , noise is added to the data at each timestep .
| (1) |
where determines noise level while represents Gaussian noise. The data becomes increasingly noisy with time, reaching at . Reverse diffusion then reconstructs the original data as follows, where and refer to the learnable mean and covariance:
| (2) |
Diffusion Transformer
In our study, we consider two types of DiT models processing different visual information: image DiT and video DiT, presented in Figure 3 (a). ❶ Image DiT model follows Vision Transformer, each block contains self-attention, cross-attention, and feed-forward networks (FFN), where the cross-attention module incorporates text and timestep conditions. ❷ Video DiT adopts dual-subblock architecture: the spatial Transformer subblock processes tokens within the same time frame, while temporal subblock manages cross-frame relationships. These blocks alternate in sequence, with cross-attention integrating conditional inputs. A complete video DiT block pairs one spatial and one temporal component in an interleaved pattern, following [19, 47].
3.2 Visualizing the Feature Smoothness of DiT
Section 1 introduces the concept of feature smoothness, and provides an intuitive explanation. In this section, we will detail the feature smoothness visualization and analysis.
To use this approach, we denote the original module parameters of the base model as in the graph and choose two random direction vectors, and , each direction vectors share the same dimension as . Firstly, these directions are normalized according to the original parameters it correspond to. We take and to disturb the model with strength coefficients and . After updating, we get a new model with parameter :
| (3) |
We denote the predicted noise after denoising steps of model before and after adding disturbs as and . We then define the feature difference with function;
| (4) |
we then plot the feature difference surface feature according to the 2-D function:
| (5) |
This approach was employed in [8] and [14], where represents the loss of a model with parameters , used to analyze trajectories of various minimization methods and model structures. Similarly, [11, 13] utilized this approach to demonstrate that different optimization algorithms converge to distinct local minima in the 2D projected space.
3.3 Skip-DiT: Improving Feature Smoothness
We visualize vanilla DiT feature smoothness in Figure 2 (w/o Skip branches) which is trained and generated on Taichi dataset. We can observe drastic feature change at the two DDPM steps, indicating they are not ideal states to cache features, thus shortening the space for caching. According to the insights from [13] and Diffusion caching study utilizing U-Net residual connection feature [18], we next investigate whether DiT feature smoothness can be improved by minimal modification to its structure to insert residual property, i.e. skip branches.
A vanilla DiT can be converted into Skip-DiT by connecting shallow blocks to deep blocks with skip branches, as shown in Figure 3 (b). Let denote the input noise embedding, and represents the output at the -th layer of Skip-DiT. The architecture consists of sequential DiT blocks with skip connections. Each DiT block at block processes the features as . The -th skip branch () connects -th block to ()-th block, which can be denoted as . Given output from the start of the skip branch and from the previous layer, the skip branch aggregates them to the input to -th block as:
| (6) |
where denotes concatenation, Norm represents the layer normalization, and Linear is a linear fully-connected layer. The final output of Skip-DiT represents the processed noise output. Each skip branch creates a shortcut path that helps preserve and process information from earlier layers, enabling better gradient flow and feature reuse throughout the network. The combination of DiT blocks and skip branches allows the model to effectively learn the underlying noise distribution while maintaining stable training dynamics.
Similarly, we initialize a class-to-video DiT with skip branches and train it from scratch on Taichi dataset, then visualize its feature smoothness as shown in Figure 2 (w/ Skip branches). The results show Skip-DiT has a more flat feature-changing landscape at both the beginning and finalizing timesteps. This justifies our initiative to enhance DiT feature smoothness with skip connections.
3.4 Skip-Cache: Caching with Skip Branches
As we have shown better feature smoothness in Skip-DiT, we can use the feature stability and skip branch property of Skip-DiT to implement efficient DiT caching, namely Skip-Cache. The inference process for global timestep of full inference can be expressed as follows:
| (7) |
Consider timesteps and where the model generates conditioned on . During this step, we cache the intermediate output from the last second layer as
| (8) |
For local timestep , with cached feature and first skip branch , the inference process can be formulated as:
| (9) |
At each local timestep, only -th and -th blocks are executed while reusing cached features from the previous global timestep through the skip branch, significantly reducing computational overhead while maintaining generation quality. This can be extended to 1: inference pattern where will be reused for the next -1 timesteps.
4 Experiments
4.1 Implementation Details
| Method | UCF101 | FFS | Sky | Taichi | FLOPs (T) | Latency (s) | Speedup | ||||
|---|---|---|---|---|---|---|---|---|---|---|---|
| FVD () | FID () | FVD () | FID () | FVD () | FID () | FVD () | FID () | ||||
| Latte | 155.22 | 22.97 | 28.88 | 5.36 | 49.46 | 11.51 | 166.84 | 11.57 | 278.63 | 9.90 | 1.00 |
| -DiT | 161.62 | 25.33 | 25.80 | 4.46 | 51.70 | 11.67 | 188.39 | 12.09 | 226.10 | 8.09 | 1.22 |
| FORA | 160.52 | 23.52 | 27.23 | 4.64 | 52.90 | 11.96 | 198.56 | 13.68 | 240.26 | 9.00 | 1.10 |
| PAB23 | 213.50 | 30.96 | 58.15 | 5.94 | 96.97 | 16.38 | 274.90 | 16.05 | 233.87 | 7.63 | 1.30 |
| PAB35 | 1176.57 | 93.30 | 863.18 | 128.34 | 573.72 | 55.66 | 828.40 | 42.96 | 222.90 | 7.14 | 1.39 |
| Skip-Cache | |||||||||||
| Skip-DiT | 141.30 | 23.78 | 20.62 | 4.32 | 49.21 | 11.92 | 163.03 | 13.55 | 290.05 | 10.02 | 1.00 |
| 141.42 | 21.46 | 23.55 | 4.49 | 51.13 | 12.66 | 167.54 | 13.89 (0.34) | 180.68 | 6.40 | 1.56 | |
| 137.98 | 19.93 | 26.76 | 4.75 | 54.17 | 13.11 | 179.43 | 14.53 | 145.87 | 5.24 | 1.91 | |
| 143.00 | 19.03 | 30.19 | 5.18 | 57.36 | 13.77 | 188.44 | 14.38 | 125.99 | 4.57 | 2.19 | |
| 145.39 | 18.72 | 35.52 | 5.86 | 62.92 | 14.18 | 209.38 | 15.20 | 121.02 | 4.35 | 2.30 | |
| 151.77 | 18.78 | 42.41 | 6.42 | 68.96 | 15.16 | 208.04 | 15.78 | 111.07 | 4.12 | 2.43 |
Models
To demonstrate the remarkable effectiveness of Skip-DiT and Skip-Cache in video generation, we employ the classic and open-source DiT model, Latte [19], as our base model. Latte consists of spatial and temporal transformer blocks, making it suitable for both class-to-video and text-to-video tasks. Hunyuan-DiT [16], the first text-to-image DiT model with skip branches, modifies the model structure by splitting transformer blocks into encoder and decoder blocks, which are connected via long skip connections, similar to UNets. In this work, we leverage Hunyuan-DiT to investigate the effectiveness of skip connections in text-to-image tasks. Additionally, we modify the structure of Latte, following the guidance in Figure 3, to evaluate the performance of skip connections in video generation tasks and explore techniques for integrating skip connections into a pre-trained DiT model.
Datasets
In the class-to-video task, we conduct comprehensive experiments on four public datasets: FaceForensics [28], SkyTimelapse [39], UCF101 [34], and Taichi-HD [31]. Following the experimental settings in Latte, we extract 16-frame video clips from these datasets and resize all frames to a resolution of 256256.
For the text-to-video task, original Latte is trained on Webvid-10M [1] and Vimeo-2M [36], comprising approximately 330k text-video pairs in total. Considering that the resolution of Webvid-10M is lower than 512512 and is predominantly used in the early stages of pre-training [20], we utilize only Vimeo-2M for training Skip-DiT. To align with Latte, we sample 330k text-video pairs from Vimeo-2M. All training data are resized to a resolution of 512512, with 16 frames per video and a frame rate of 8.
Training Details
For the training of class-to-video tasks, we train all instances of Skip-DiT from scratch without any initialization. During training, we update all parameters in Skip-DiT, including skip branches. For text-to-video tasks, we propose a two-stage continual training strategy as follows:
-
•
Skip-branch training: The models are initialized with the weights of the original text-to-video Latte model, while the weight of skip branches are initialized randomly. During this stage, we train only the skip branches until the model can roughly generate items. This stage takes approximately one day.
-
•
Overall training: After fully training the skip branches, we unfreeze all other parameters and perform overall training. At this stage, Skip-DiT rapidly recovers its generation capability within approximately two days and can generate content comparable to the original Latte with an additional three days of training.
This strategy significantly reduces training costs compared to training from scratch. All our training experiments are conducted on 8 H100 GPUs, employing the video-image joint training strategy proposed in Ma et al. [19]. We find that this approach significantly enhances training stability.
Evaluation Details
Following previous works [47, 18] and Latte, we evaluate text-to-video models using VBench [10], Peak Signal-to-Noise Ratio (PSNR), Learned Perceptual Image Patch Similarity (LPIPS) [43], and Structural Similarity Index Measure (SSIM) [37]. VBench is a comprehensive benchmark suite comprising 16 evaluation dimensions. PSNR is a widely used metric for assessing the quality of image reconstruction, LPIPS measures feature distances extracted from pre-trained networks, and SSIM evaluates structural information differences. All the videos generated for evaluation are sampled with 50 steps DDIM [33], which is the default setting used in Latte.
For class-to-image tasks, we evaluate the similarity between generated and real videos using Fréchet Video Distance (FVD) [35] and Fréchet Inception Distance (FID) [21], following the evaluation guidelines of StyleGAN-V [42]. Latte uses 250-step DDPM [9] as the default solver for class-to-video tasks, which we adopt for all tasks except UCF101. For UCF101, we employ 50-step DDIM [33], as it outperforms 250-step DDPM on both Latte and Skip-DiT. Table 2 highlights this phenomenon, showing our methods consistently outperform DDPM-250 under comparable throughput, except for UCF101, where DDIM performs better than 250 steps DDPM.
Implementation Details of Other Caching Methods
We compare with 4 other DiT caching methods in video and image generation: ❶ T-GATE [44] reuses self-attention in the semantics-planning phase and skips cross-attention in the fidelity-improving phase. We follow Zhao et al. [47] to split these two phases. ❷ -DiT identifies high similarity in deviations between feature maps and reuses them at the next timestep. While this method is originally designed for images, we extend it to video DiTs by caching only the front blocks, as we observe significant degeneration when caching the back blocks. ❸ FORA [30] reuses attention features across timesteps. ❹ PAB [47] further extends it by broadcasting cross, spatial, and temporal attention features separately. All these caching methods are performed on Latte for equal comparison with Skip-Cache on Skip-DiT .
4.2 Main Results
Class-to-video Generation
We compare the quantitative performance of Latte and Skip-DiT on four class-to-video tasks, as shown in Table 1. Skip-DiT consistently outperforms Latte in terms of FVD scores across all tasks while achieving comparable performance in FID scores, demonstrating its strong video generation capabilities. Furthermore, we observe that Skip-Cache significantly outperforms other caching methods across most metrics, incurring only an average loss of in the FVD score and in the FID score while achieving a 1.56 speedup. In comparison, only PAB [47] achieves a speedup of more than 1.3, but at the cost of a substantial average loss of in FVD score and in FID score. Notably, in the Taichi [31] task, all other caching methods exhibit significant degradation in FVD scores( 21.5), whereas Skip-Cache experiences only a slight loss (163.06167.54). To achieve even higher speedup on the other three class-to-video tasks, we accelerated Skip-Cache with a larger cache timesteps (N=3), resulting in a 2.19 speedup with an average loss of in FVD and a improvement in FID.
| Method | UCF101 | FFS | Sky | Taichi | ||||
|---|---|---|---|---|---|---|---|---|
| FVD | FID | FVD | FID | FVD | FID | FVD | FID | |
| Latte | 165.04 | 23.75 | 28.88 | 5.36 | 49.46 | 11.51 | 166.84 | 11.57 |
| Skip-DiT | 173.70 | 22.95 | 20.62 | 4.32 | 49.22 | 12.05 | 163.03 | 13.55 |
| Skip-Cachen=2 | 165.60 | 22.73 | 23.55 | 4.49 | 51.13 | 12.66 | 167.54 | 13.89 |
| DDIM+Skip-DiT | 134.22 | 24.60 | 37.28 | 6.48 | 86.39 | 13.67 | 343.97 | 21.01 |
| DDIM+Latte | 146.78 | 23.06 | 39.10 | 6.47 | 78.38 | 13.73 | 321.97 | 21.86 |
| Skip-Cachen=3 | 169.37 | 22.47 | 26.76 | 4.75 | 54.17 | 13.11 | 179.43 | 14.53 |
| DDIM+Skip-DiT | 139.52 | 24.71 | 39.20 | 6.49 | 90.62 | 13.80 | 328.47 | 21.33 |
| DDIM | 148.46 | 23.41 | 41.00 | 6.54 | 74.39 | 14.20 | 327.22 | 22.96 |
| Method | VBench(%) | PSNR | LPIPS | SSIM | FLOPs (T) | latency (s) | speedup |
|---|---|---|---|---|---|---|---|
| Latte | 76.14 | – | – | – | 1587.25 | 27.11 | 1.00 |
| T-GATE | 75.68 (0.46) | 22.78 | 0.19 | 0.78 | 1470.72 | 24.15 | 1.12 |
| -DiT | 76.06 (0.08) | 24.01 | 0.17 | 0.81 | 1274.36 | 21.40 | 1.27 |
| FORA | 76.06 (0.08) | 22.93 | 0.14 | 0.79 | 1341.72 | 24.21 | 1.19 |
| PAB235 | 73.79 (2.35) | 19.18 | 0.27 | 0.66 | 1288.08 | 23.24 | 1.24 |
| PAB347 | 72.08 (4.06) | 18.20 | 0.32 | 0.63 | 1239.35 | 22.23 | 1.29 |
| PAB469 | 71.64 (4.50) | 17.40 | 0.35 | 0.60 | 1210.11 | 21.60 | 1.33 |
| Skip-DiT | 75.60 | – | – | – | 1648.13 | 28.72 | 1.00 |
| Skip-Cache 75% | |||||||
| 75.36 (0.24) | 26.02 | 0.10 | 0.84 | 1066.62 | 18.25 | 1.57 | |
| 75.07 (0.53) | 22.85 | 0.18 | 0.76 | 852.38 | 14.88 | 1.93 | |
| 74.43 (1.17) | 22.08 | 0.22 | 0.73 | 760.56 | 13.03 | 2.20 | |
| Skip-Cache 65% | |||||||
| 75.51 (0.09) | 29.52 | 0.06 | 0.89 | 1127.83 | 19.28 | 1.49 | |
| 75.26 (0.34) | 27.46 | 0.09 | 0.85 | 974.80 | 16.67 | 1.72 | |
| 74.73 (0.87) | 25.97 | 0.13 | 0.81 | 882.98 | 15.12 | 1.90 |
Text-to-video Generation
| Method | FID () | CLIP () | PSNR () | LPIPS () | SSIM () | FLOPs (T) | latency (s) | speedup |
|---|---|---|---|---|---|---|---|---|
| HunYuan-DiT | 32.64 | 30.51 | – | – | – | 514.02 | 18.69 | 1.00 |
| TGATE | 32.71 | 30.64 | 16.80 | 0.24 | 0.61 | 378.94 | 13.21 | 1.41 |
| -Cache | 28.35 | 30.35 | 16.56 | 0.21 | 0.65 | 362.67 | 13.58 | 1.38 |
| FORA | 31.21 | 30.53 | 19.58 | 0.14 | 0.75 | 330.68 | 13.20 | 1.42 |
| Skip-Cache | ||||||||
| 31.30 | 30.52 | 22.09 | 0.10 | 0.84 | 348.24 | 12.76 | 1.46 | |
| 29.53 | 30.55 | 21.25 | 0.11 | 0.81 | 299.48 | 10.91 | 1.71 | |
| 27.49 | 30.55 | 20.55 | 0.13 | 0.78 | 270.22 | 10.02 | 1.87 | |
| 28.37 | 30.56 | 19.94 | 0.14 | 0.76 | 260.47 | 9.51 | 1.96 | |
| 27.21 | 30.71 | 19.18 | 0.18 | 0.70 | 240.96 | 8.96 | 2.09 |
In Table 3, we present a quantitative evaluation of all text-to-video models and caching methods. Videos are generated using the prompts from VBench [10], which is considered a more generalized benchmark [47, 10, 48]. Compared with the original Latte, Skip-Cache achieves a comparable VBench score (75.60 vs. 76.14) with only six days of continual pre-training on 330k training samples.
To demonstrate the superiority of the caching mechanism in Skip-Cache, we evaluate two caching settings: caching at timesteps 700–50 and 800–50 (out of 1000 timesteps in total). In both settings, Skip-Cache achieves the highest speedup while maintaining superior scores in PSNR, LPIPS, and SSIM, with only a minor loss in VBench score.
In the first setting, our caching mechanism achieves 1.49 and 1.72 speedup with only a 0.12% and 0.22% loss in VBench score, respectively. Among other caching methods, only PAB469 achieves a speedup of more than 1.30, but at the cost of a 4.50% drop in VBench score. Moreover, our caching method can achieve a 1.90 speedup while still maintaining absolutely better PSNR, LPIPS, and SSIM scores compared to other caching methods.
Furthermore, in the second setting, we achieve a 2.20 speedup with just a 1.17% sacrifice in VBench score, representing the highest speedup among current training-free DiT acceleration works.
4.3 Generalize Skip-Cache to image Generation
Hunyuan-DiT [16] is a powerful text-to-image DiT model featuring skip branches, whose effectiveness has been demonstrated in [16]. However, its skip branches have not been explored for accelerating image generation. We leverage these skip branches using the same caching mechanism as Skip-Cache and compare our caching strategy with other training-free acceleration methods. Furthermore, we extend our proposed Skip-Cache to class-to-image task in Appendix 6, where Skip-DiT exceeds vanilla DiT model with only around 38% of its training cost.
Evaluation Details
To evaluate the generalization of the caching mechanism in Skip-Cache for text-to-image tasks, we use the zero-shot Fréchet Inception Distance (FID) on the MS COCO [17] 256256 validation dataset by generating 30,000 images based on its prompts, following the evaluation guidelines established by Hunyuan-DiT. Additionally, we employ Peak Signal-to-Noise Ratio (PSNR), Learned Perceptual Image Patch Similarity (LPIPS) [43], and Structural Similarity Index Measure (SSIM) [37] to assess the changes introduced by the caching methods. To ensure a fair comparison, we disable the prompt enhancement feature of Hunyuan-DiT. All videos are generated at a resolution of 10241024 and subsequently resized to 256256 for evaluation.
Evaluation Results
Table 4 provides a comprehensive comparison of Hunyuan-DiT and various caching methods. Notably, our caching mechanism achieves a 2.28 speedup without any degradation in FID or CLIP scores. Furthermore, it outperforms all other caching methods in terms of PSNR, LPIPS, and SSIM scores, consistently maintaining the highest performance even with a 1.93 speedup. These findings underscore the robustness and adaptability of our caching mechanism to image generation tasks.
| Caching Timestep | VBench (%) | PSNR | LPIPS | SSIM |
|---|---|---|---|---|
| 70050 | 75.51 | 29.52 | 0.06 | 0.89 |
| 950300 | 75.48 | 20.58 | 0.23 | 0.73 |
| 80050 | 75.36 | 26.02 | 0.10 | 0.84 |
| 90050 | 75.24 | 22.13 | 0.19 | 0.76 |
| Method | VBench (%) | PSNR | LPIPS | SSIM |
|---|---|---|---|---|
| Skip-DiT | 75.60 | – | – | – |
| T-GATE | 75.16 | 24.09 | 0.16 | 0.78 |
| -DiT | 75.48 | 22.64 | 0.17 | 0.79 |
| FORA | 75.38 | 23.26 | 0.16 | 0.79 |
| PAB235 | 73.79 | 19.92 | 0.29 | 0.68 |
| PAB347 | 72.08 | 18.98 | 0.34 | 0.65 |
| PAB469 | 71.64 | 18.02 | 0.37 | 0.62 |
4.4 Ablation studies
Select the best timesteps to cache
A heat map visualizing feature dynamics across blocks is shown in Figure 4. After incorporating skip branches into Latte, we observe that major changes are concentrated in the early timesteps, with feature dynamics becoming considerably smoother in the later timesteps (70050). In contrast, features in Latte exhibit rapid changes across all timesteps. This finding highlights that caching during smoother timesteps leads to significantly better performance, supporting the hypothesis that smooth features enhance caching efficiency in DiT. In Table 5, we further validate this observation: under equivalent throughput, caching in the later timesteps (70050), where features are smoother, outperforms caching in the earlier timesteps (950300), achieving superior PSNR, LPIPS, and SSIM scores. Additionally, we segmented the rapidly changing timesteps and experimented with three caching ranges: 90050, 80050, and 70050. The results show that increasing the ratio of smoother regions significantly improves caching performance. These findings underscore the importance of leveraging smoother feature dynamics for optimal caching.
Compatibility of Skip-DiT with other caching methods
As shown in Table 6, we extend the existing DiT caching methods to Skip-DiT and observe slight performance improvements. Specially, in -DiT, the middle blocks are cached instead of the front blocks, as discussed in Section 4.1, to leverage Skip-DiT’ U-shaped structure. Taking PAB [47] as an example, it loses 1.15% less in VBench score and achieves noticeably better PSNR and SSIM scores on Skip-DiT compared to Latte, highlighting the potential of Skip-DiT to enhance cache-based methods, and proving the superior caching efficiency of model with better feature smoothness.
5 Conclusion
In this work, we introduce Skip-DiT, a skip-branch-enhanced DiT model designed to produce smoother features and propose Skip-DiT cache to improve caching efficiency in video and image generation tasks. By enhancing feature smoothness across timesteps, Skip-DiT unlocks the potential to cache most blocks while maintaining high-generation quality. Additionally, Skip-Cache leverages its U-net-style architecture to enable cross-timestep feature caching. Our approach achieves maximum speedup in cache-based visual DiT generation while preserving the highest similarity to original outputs. Furthermore, we analyze feature dynamics before and after incorporating skip branches, demonstrating the effectiveness of caching at timesteps with smoother features. We also show that Skip-DiT is compatible with other caching methods, further extending its applicability. Overall, Skip-DiT can seamlessly integrate with various DiT backbones, enabling real-time, high-quality video and image generation while consistently outperforming baseline methods. We believe Skip-Cache offers a simple yet powerful foundation for advancing future research and practical applications in visual generation.
References
- Bain et al. [2021] Max Bain, Arsha Nagrani, Gül Varol, and Andrew Zisserman. Frozen in time: A joint video and image encoder for end-to-end retrieval. In Proceedings of the IEEE/CVF international conference on computer vision, pages 1728–1738, 2021.
- Bao et al. [2022] Fan Bao, Shen Nie, Kaiwen Xue, Yue Cao, Chongxuan Li, Hang Su, and Jun Zhu. All are worth words: A vit backbone for diffusion models. 2023 IEEE/CVF Conference on Computer Vision and Pattern Recognition (CVPR), pages 22669–22679, 2022.
- [3] James Betker, Gabriel Goh, Li Jing, † TimBrooks, Jianfeng Wang, Linjie Li, † LongOuyang, † JuntangZhuang, † JoyceLee, † YufeiGuo, † WesamManassra, † PrafullaDhariwal, † CaseyChu, † YunxinJiao, and Aditya Ramesh. Improving image generation with better captions.
- Chen et al. [2023] Junsong Chen, Jincheng Yu, Chongjian Ge, Lewei Yao, Enze Xie, Yue Wu, Zhongdao Wang, James T. Kwok, Ping Luo, Huchuan Lu, and Zhenguo Li. Pixart-: Fast training of diffusion transformer for photorealistic text-to-image synthesis. ArXiv, abs/2310.00426, 2023.
- Chen et al. [2024a] Lei Chen, Yuan Meng, Chen Tang, Xinzhu Ma, Jingyan Jiang, Xin Wang, Zhi Wang, and Wenwu Zhu. Q-dit: Accurate post-training quantization for diffusion transformers. arXiv preprint arXiv:2406.17343, 2024a.
- Chen et al. [2024b] Pengtao Chen, Mingzhu Shen, Peng Ye, Jianjian Cao, Chongjun Tu, Christos-Savvas Bouganis, Yiren Zhao, and Tao Chen. Delta-dit: A training-free acceleration method tailored for diffusion transformers. arXiv preprint arXiv:2406.01125, 2024b.
- Deng et al. [2009] Jia Deng, Wei Dong, Richard Socher, Li-Jia Li, Kai Li, and Li Fei-Fei. Imagenet: A large-scale hierarchical image database. In 2009 IEEE conference on computer vision and pattern recognition, pages 248–255. Ieee, 2009.
- Goodfellow and Vinyals [2014] Ian J. Goodfellow and Oriol Vinyals. Qualitatively characterizing neural network optimization problems. CoRR, abs/1412.6544, 2014.
- Ho et al. [2020] Jonathan Ho, Ajay Jain, and Pieter Abbeel. Denoising diffusion probabilistic models. In Advances in Neural Information Processing Systems 33: Annual Conference on Neural Information Processing Systems 2020, NeurIPS 2020, December 6-12, 2020, virtual, 2020.
- Huang et al. [2024] Ziqi Huang, Yinan He, Jiashuo Yu, Fan Zhang, Chenyang Si, Yuming Jiang, Yuanhan Zhang, Tianxing Wu, Qingyang Jin, Nattapol Chanpaisit, Yaohui Wang, Xinyuan Chen, Limin Wang, Dahua Lin, Yu Qiao, and Ziwei Liu. Vbench: Comprehensive benchmark suite for video generative models. In Proceedings of the IEEE/CVF Conference on Computer Vision and Pattern Recognition (CVPR), pages 21807–21818, 2024.
- Im et al. [2016] Daniel Jiwoong Im, Michael Tao, and Kristin Branson. An empirical analysis of deep network loss surfaces. ArXiv, abs/1612.04010, 2016.
- Lab and etc. [2024] PKU-Yuan Lab and Tuzhan AI etc. Open-sora-plan, 2024.
- Li et al. [2017] Hao Li, Zheng Xu, Gavin Taylor, and Tom Goldstein. Visualizing the loss landscape of neural nets. ArXiv, abs/1712.09913, 2017.
- Li et al. [2018] Hao Li, Zheng Xu, Gavin Taylor, Christoph Studer, and Tom Goldstein. Visualizing the loss landscape of neural nets. In Advances in Neural Information Processing Systems 31: Annual Conference on Neural Information Processing Systems 2018, NeurIPS 2018, December 3-8, 2018, Montréal, Canada, pages 6391–6401, 2018.
- Li et al. [2023] Senmao Li, Taihang Hu, Fahad Shahbaz Khan, Linxuan Li, Shiqi Yang, Yaxing Wang, Ming-Ming Cheng, and Jian Yang. Faster diffusion: Rethinking the role of unet encoder in diffusion models. arXiv preprint arXiv:2312.09608, 2023.
- Li et al. [2024] Zhimin Li, Jianwei Zhang, Qin Lin, Jiangfeng Xiong, Yanxin Long, Xinchi Deng, Yingfang Zhang, Xingchao Liu, Minbin Huang, Zedong Xiao, Dayou Chen, Jiajun He, Jiahao Li, Wenyue Li, Chen Zhang, Rongwei Quan, Jianxiang Lu, Jiabin Huang, Xiaoyan Yuan, Xiaoxiao Zheng, Yixuan Li, Jihong Zhang, Chao Zhang, Meng Chen, Jie Liu, Zheng Fang, Weiyan Wang, Jinbao Xue, Yangyu Tao, Jianchen Zhu, Kai Liu, Sihuan Lin, Yifu Sun, Yun Li, Dongdong Wang, Mingtao Chen, Zhichao Hu, Xiao Xiao, Yan Chen, Yuhong Liu, Wei Liu, Di Wang, Yong Yang, Jie Jiang, and Qinglin Lu. Hunyuan-dit: A powerful multi-resolution diffusion transformer with fine-grained chinese understanding, 2024.
- Lin et al. [2014] Tsung-Yi Lin, Michael Maire, Serge J. Belongie, James Hays, Pietro Perona, Deva Ramanan, Piotr Dollár, and C. Lawrence Zitnick. Microsoft COCO: common objects in context. In Computer Vision - ECCV 2014 - 13th European Conference, Zurich, Switzerland, September 6-12, 2014, Proceedings, Part V, pages 740–755. Springer, 2014.
- Ma et al. [2023] Xinyin Ma, Gongfan Fang, and Xinchao Wang. Deepcache: Accelerating diffusion models for free. 2024 IEEE/CVF Conference on Computer Vision and Pattern Recognition (CVPR), pages 15762–15772, 2023.
- Ma et al. [2024] Xin Ma, Yaohui Wang, Gengyun Jia, Xinyuan Chen, Ziwei Liu, Yuan-Fang Li, Cunjian Chen, and Yu Qiao. Latte: Latent diffusion transformer for video generation. arXiv preprint arXiv:2401.03048, 2024.
- OpenAI [2024] OpenAI. Sora: Creating video from text. https://openai.com/sora, 2024.
- Parmar et al. [2021] Gaurav Parmar, Richard Zhang, and Jun-Yan Zhu. On buggy resizing libraries and surprising subtleties in FID calculation. CoRR, abs/2104.11222, 2021.
- Peebles and Xie [2023] William Peebles and Saining Xie. Scalable diffusion models with transformers. In IEEE/CVF International Conference on Computer Vision, ICCV 2023, Paris, France, October 1-6, 2023, pages 4172–4182. IEEE, 2023.
- Peebles and Xie [2022] William S. Peebles and Saining Xie. Scalable diffusion models with transformers. 2023 IEEE/CVF International Conference on Computer Vision (ICCV), pages 4172–4182, 2022.
- Podell et al. [2023] Dustin Podell, Zion English, Kyle Lacey, A. Blattmann, Tim Dockhorn, Jonas Muller, Joe Penna, and Robin Rombach. Sdxl: Improving latent diffusion models for high-resolution image synthesis. ArXiv, abs/2307.01952, 2023.
- Polyak et al. [2024] Adam Polyak, Amit Zohar, Andrew Brown, Andros Tjandra, Animesh Sinha, Ann Lee, Apoorv Vyas, Bowen Shi, Chih-Yao Ma, Ching-Yao Chuang, David Yan, Dhruv Choudhary, Dingkang Wang, Geet Sethi, Guan Pang, Haoyu Ma, Ishan Misra, Ji Hou, Jialiang Wang, Kiran Jagadeesh, Kunpeng Li, Luxin Zhang, Mannat Singh, Mary Williamson, Matt Le, Matthew Yu, Mitesh Kumar Singh, Peizhao Zhang, Peter Vajda, Quentin Duval, Rohit Girdhar, Roshan Sumbaly, Sai Saketh Rambhatla, Sam Tsai, Samaneh Azadi, Samyak Datta, Sanyuan Chen, Sean Bell, Sharadh Ramaswamy, Shelly Sheynin, Siddharth Bhattacharya, Simran Motwani, Tao Xu, Tianhe Li, Tingbo Hou, Wei-Ning Hsu, Xi Yin, Xiaoliang Dai, Yaniv Taigman, Yaqiao Luo, Yen-Cheng Liu, Yi-Chiao Wu, Yue Zhao, Yuval Kirstain, Zecheng He, Zijian He, Albert Pumarola, Ali Thabet, Artsiom Sanakoyeu, Arun Mallya, Baishan Guo, Boris Araya, Breena Kerr, Carleigh Wood, Ce Liu, Cen Peng, Dimitry Vengertsev, Edgar Schonfeld, Elliot Blanchard, Felix Juefei-Xu, Fraylie Nord, Jeff Liang, John Hoffman, Jonas Kohler, Kaolin Fire, Karthik Sivakumar, Lawrence Chen, Licheng Yu, Luya Gao, Markos Georgopoulos, Rashel Moritz, Sara K. Sampson, Shikai Li, Simone Parmeggiani, Steve Fine, Tara Fowler, Vladan Petrovic, and Yuming Du. Movie Gen: A Cast of Media Foundation Models. arXiv e-prints, art. arXiv:2410.13720, 2024.
- Rombach et al. [2021] Robin Rombach, A. Blattmann, Dominik Lorenz, Patrick Esser, and Björn Ommer. High-resolution image synthesis with latent diffusion models. 2022 IEEE/CVF Conference on Computer Vision and Pattern Recognition (CVPR), pages 10674–10685, 2021.
- Ronneberger et al. [2015] Olaf Ronneberger, Philipp Fischer, and Thomas Brox. U-net: Convolutional networks for biomedical image segmentation. ArXiv, abs/1505.04597, 2015.
- Rössler et al. [2018] Andreas Rössler, Davide Cozzolino, Luisa Verdoliva, Christian Riess, Justus Thies, and Matthias Nießner. Faceforensics: A large-scale video dataset for forgery detection in human faces. CoRR, abs/1803.09179, 2018.
- Sauer et al. [2023] Axel Sauer, Dominik Lorenz, Andreas Blattmann, and Robin Rombach. Adversarial diffusion distillation. arXiv preprint arXiv:2311.17042, 2023.
- Selvaraju et al. [2024] Pratheba Selvaraju, Tianyu Ding, Tianyi Chen, Ilya Zharkov, and Luming Liang. Fora: Fast-forward caching in diffusion transformer acceleration. arXiv preprint arXiv:2407.01425, 2024.
- Siarohin et al. [2019] Aliaksandr Siarohin, Stéphane Lathuilière, Sergey Tulyakov, Elisa Ricci, and Nicu Sebe. First order motion model for image animation. In Advances in Neural Information Processing Systems 32: Annual Conference on Neural Information Processing Systems 2019, NeurIPS 2019, December 8-14, 2019, Vancouver, BC, Canada, pages 7135–7145, 2019.
- So et al. [2023] Junhyuk So, Jungwon Lee, and Eunhyeok Park. Frdiff: Feature reuse for exquisite zero-shot acceleration of diffusion models. arXiv preprint arXiv:2312.03517, 2023.
- Song et al. [2021] Jiaming Song, Chenlin Meng, and Stefano Ermon. Denoising diffusion implicit models. In 9th International Conference on Learning Representations, ICLR 2021, Virtual Event, Austria, May 3-7, 2021. OpenReview.net, 2021.
- Soomro et al. [2012] Khurram Soomro, Amir Roshan Zamir, and Mubarak Shah. UCF101: A dataset of 101 human actions classes from videos in the wild. CoRR, abs/1212.0402, 2012.
- Unterthiner et al. [2018] Thomas Unterthiner, Sjoerd van Steenkiste, Karol Kurach, Raphaël Marinier, Marcin Michalski, and Sylvain Gelly. Towards accurate generative models of video: A new metric & challenges. CoRR, abs/1812.01717, 2018.
- Wang et al. [2023] Yaohui Wang, Xinyuan Chen, Xin Ma, Shangchen Zhou, Ziqi Huang, Yi Wang, Ceyuan Yang, Yinan He, Jiashuo Yu, Peiqing Yang, Yuwei Guo, Tianxing Wu, Chenyang Si, Yuming Jiang, Cunjian Chen, Chen Change Loy, Bo Dai, Dahua Lin, Yu Qiao, and Ziwei Liu. LAVIE: high-quality video generation with cascaded latent diffusion models. CoRR, abs/2309.15103, 2023.
- Wang and Bovik [2002] Zhou Wang and Alan C. Bovik. A universal image quality index. IEEE Signal Process. Lett., 9(3):81–84, 2002.
- Wimbauer et al. [2024] Felix Wimbauer, Bichen Wu, Edgar Schoenfeld, Xiaoliang Dai, Ji Hou, Zijian He, Artsiom Sanakoyeu, Peizhao Zhang, Sam Tsai, Jonas Kohler, et al. Cache me if you can: Accelerating diffusion models through block caching. In Proceedings of the IEEE/CVF Conference on Computer Vision and Pattern Recognition, pages 6211–6220, 2024.
- Xiong et al. [2018] Wei Xiong, Wenhan Luo, Lin Ma, Wei Liu, and Jiebo Luo. Learning to generate time-lapse videos using multi-stage dynamic generative adversarial networks. In 2018 IEEE Conference on Computer Vision and Pattern Recognition, CVPR 2018, Salt Lake City, UT, USA, June 18-22, 2018, pages 2364–2373. Computer Vision Foundation / IEEE Computer Society, 2018.
- Yang et al. [2024] Zhuoyi Yang, Jiayan Teng, Wendi Zheng, Ming Ding, Shiyu Huang, Jiazheng Xu, Yuanming Yang, Wenyi Hong, Xiaohan Zhang, Guanyu Feng, et al. Cogvideox: Text-to-video diffusion models with an expert transformer. arXiv preprint arXiv:2408.06072, 2024.
- Yin et al. [2024] Tianwei Yin, Michaël Gharbi, Richard Zhang, Eli Shechtman, Fredo Durand, William T Freeman, and Taesung Park. One-step diffusion with distribution matching distillation. In Proceedings of the IEEE/CVF Conference on Computer Vision and Pattern Recognition, pages 6613–6623, 2024.
- Yu et al. [2022] Sihyun Yu, Jihoon Tack, Sangwoo Mo, Hyunsu Kim, Junho Kim, Jung-Woo Ha, and Jinwoo Shin. Generating videos with dynamics-aware implicit generative adversarial networks. In International Conference on Learning Representations, 2022.
- Zhang et al. [2018] Richard Zhang, Phillip Isola, Alexei A Efros, Eli Shechtman, and Oliver Wang. The unreasonable effectiveness of deep features as a perceptual metric. In Proceedings of the IEEE conference on computer vision and pattern recognition, pages 586–595, 2018.
- Zhang et al. [2024a] Wentian Zhang, Haozhe Liu, Jinheng Xie, Francesco Faccio, Mike Zheng Shou, and Jürgen Schmidhuber. Cross-attention makes inference cumbersome in text-to-image diffusion models. CoRR, abs/2404.02747, 2024a.
- Zhang et al. [2024b] Wentian Zhang, Haozhe Liu, Jinheng Xie, Francesco Faccio, Mike Zheng Shou, and Jürgen Schmidhuber. Cross-attention makes inference cumbersome in text-to-image diffusion models. arXiv preprint arXiv:2404.02747, 2024b.
- Zhang et al. [2024c] Yiming Zhang, Zhening Xing, Yanhong Zeng, Youqing Fang, and Kai Chen. Pia: Your personalized image animator via plug-and-play modules in text-to-image models. In Proceedings of the IEEE/CVF Conference on Computer Vision and Pattern Recognition, pages 7747–7756, 2024c.
- Zhao et al. [2024] Xuanlei Zhao, Xiaolong Jin, Kai Wang, and Yang You. Real-time video generation with pyramid attention broadcast, 2024.
- Zheng et al. [2024] Zangwei Zheng, Xiangyu Peng, Tianji Yang, Chenhui Shen, Shenggui Li, Hongxin Liu, Yukun Zhou, Tianyi Li, and Yang You. Open-sora: Democratizing efficient video production for all, 2024.
Supplementary Material
6 Class-to-image Generation Experiments
Peebles and Xie [22] proposed the first diffusion model based on the transformer architecture, and it outperforms all prior diffusion models on the class conditional ImageNet [7] 512512 and 256256 benchmarks. We add skip branches to its largest model DiT-XL/2 to get Skip-DiT. We train Skip-DiT on class conditional ImageNet with resolution 256256 from scratch with completely the same expirements setting as DiT-XL/2, and far exceeds DiT-XL/2 with only around 38% of its training cost.
Training of Skip-DiT
We modify the structure of DiT-XL/2 following the methodology outlined in Section 3 and train Skip-DiT for 2,900,000 steps on 8 A100 GPUs, compared to 7,000,000 steps for DiT-XL/2, which also uses 8 A100 GPUs. The datasets and other training settings remain identical to those used for DiT-XL/2, and we utilize the official training code of DiT-XL/2***https://github.com/facebookresearch/DiT. The performance comparison is presented in Table 7, which demonstrates that Skip-DiT significantly outperforms DiT-XL/2 while requiring only 38% of its training steps, highlighting the training efficiency and effectiveness of Skip-DiT.
| Model | Steps | FID | sFID | IS | Precision | Recall |
|---|---|---|---|---|---|---|
| cfg=1.0 | ||||||
| DiT-XL/2 | 7000k | 9.49 | 7.17 | 122.49 | 0.67 | 0.68 |
| Skip-DiT | 2900k | 8.37 | 6.50 | 127.63 | 0.68 | 0.68 |
| cfg=1.5 | ||||||
| DiT-XL/2 | 7000k | 2.30 | 4.71 | 276.26 | 0.83 | 0.58 |
| Skip-DiT | 2900k | 2.29 | 4.58 | 281.81 | 0.83 | 0.58 |
Accelerating Evaluation
| Methods | FID | sFID | IS | Precision% | Recall% | Speedup |
|---|---|---|---|---|---|---|
| cfg=1.5 | ||||||
| DiT-XL/2 | 2.30 | 4.71 | 276.26 | 82.68 | 57.65 | 1.00 |
| FORA | 2.45 | 5.44 | 265.94 | 81.21 | 58.36 | 1.57 |
| Delta-DiT | 2.47 | 5.61 | 265.33 | 81.05 | 58.83 | 1.45 |
| Skip-Cache | ||||||
| Skip-DiT | 2.29 | 4.58 | 281.81 | 82.88 | 57.53 | 1.00 |
| 2.31 | 4.76 | 277.51 | 82.52 | 58.06 | 1.46 | |
| 2.40 | 4.98 | 272.05 | 82.14 | 57.86 | 1.73 | |
| 2.54 | 5.31 | 267.34 | 81.60 | 58.31 | 1.93 | |
| cfg=1.0 | ||||||
| DiT-XL/2 | 9.49 | 7.17 | 122.49 | 66.66 | 67.69 | 1.00 |
| FORA | 11.72 | 9.27 | 113.01 | 64.46 | 67.69 | 1.53 |
| Delta-DiT | 12.03 | 9.68 | 111.86 | 64.57 | 67.53 | 1.42 |
| Skip-Cache | ||||||
| Skip-DiT | 8.37 | 6.50 | 127.63 | 68.06 | 67.89 | 1.00 |
| 9.25 | 7.09 | 123.57 | 67.32 | 67.40 | 1.46 | |
| 10.18 | 7.72 | 119.60 | 66.53 | 67.84 | 1.71 | |
| 11.37 | 8.49 | 116.01 | 65.73 | 67.32 | 1.92 |
We evaluate Skip-Cache on Skip-DiT and compare its performance against two other caching methods: -DiT and FORA. As shown in Table 8, Skip-Cache achieves a 1.46 speedup with only a minimal FID loss of when the classifier-free guidance scale is set to 1.5, compared to the 7–8 larger losses observed with -DiT and FORA. Moreover, even with a 1.9 acceleration, Skip-DiT performs better than the other caching methods. These findings further confirm the effectiveness of Skip-DiT for class-to-image tasks.
7 Evaluation Details
VBench [10]
is a novel evaluation framework for video generation models. It breaks down video generation assessment to 16 dimensions from video quality and condition consistency: subject consistency, background consistency, temporal flickering, motion smoothness, dynamic degree, aesthetic quality, imaging quality, object class, multiple objects, human action, color, spatial relationship, scene, temporal style, appearance style, overall consistency.
Peak Signal-to-Noise Ratio (PSNR)
measures generated visual content quality by comparing a processed version to the original reference by:
| (10) |
where is the maximum possible pixel value, and calculates the Mean Squared Error between original and processed images or videos. Higher PSNR indicates better reconstruction quality. However, PSNR does not always correlate with human perception and is sensitive to pixel-level changes.
Structural Similarity Index Measure (SSIM)
is a perceptual metric that evaluates image quality by considering luminance, contrast, and structure:
| (11) |
where are weights for luminance, contrast, and structure quality, where luminance comparison is , contrast comparison is , and structure comparison is , with denoting numerical stability coefficients. SSIM scores range from -1 to 1, where 1 means identical visual content.
Learned Perceptual Image Patch Similarity (LPIPS)
is a deep learning-based metric that measures perceptual similarity using L2-Norm of visual features extracted from pretrained CNN . LPIPS captures semantic similarities and is therefore more robust to small geometric transformations than PSNR and SSIM.
| (12) |
Fréchet Inception Distance (FID) and Fréchet Video Distance (FVD)
FID measures the quality and diversity of generated images by computing distance between feature distributions of reference and generated images using inception architecture CNNs, where are mean and covariance of features.
| (13) |
FVD is a video extension of FID. Lower FID and FVD indicate higher generation quality.
8 Implementation Details
DeepCache
DeepCache [18] is a training-free caching method designed for U-Net-based diffusion models, leveraging the inherent temporal redundancy in sequential denoising steps. It utilizes the skip connections of the U-Net to reuse high-level features while updating low-level features efficiently. Skip-Cache shares significant similarities with DeepCache but extends the method to DiT models. Specifically, we upgrade traditional DiT models to Skip-DiT and cache them using Skip-Cache . In the work of DeepCache, two key caching decisions are introduced: (1) N: the number of steps for reusing cached high-level features. Cached features are computed once and reused for the next N-1 steps. (2) The layer at which caching is performed. For instance, caching at the first layer ensures that only the first and last layers of the U-Net are recomputed. In Skip-Cache, we adopt these two caching strategies and additionally account for the timesteps to cache, addressing the greater complexity of DiT models compared to U-Net-based diffusion models. For all tasks except the class-to-image task, caching is performed at the first layer, whereas for the class-to-image task, it is applied at the third layer.
-DiT
-DiT [6] is a training-free caching method designed for image-generating DiT models. Instead of caching the feature maps directly, it uses the offsets of features as cache objects to preserve input information. This approach is based on the observation that the front blocks of DiT are responsible for generating the image outlines, while the rear blocks focus on finer details. A hyperparameter is introduced to denote the boundary between the outline and detail generation stages. When , -Cache is applied to the rear blocks; when , it is applied to the front blocks. The number of cached blocks is represented by .
While this caching method was initially designed for image generation tasks, we extend it to video generation tasks. In video generation, we observe significant degradation in performance when caching the rear blocks, so we restrict caching to the front blocks during the outline generation stage. For Hunyuan-DiT [16], we cache the middle blocks due to the U-shaped transformer architecture. Detailed configurations are provided in Table 9.
| -DiT | Task | Diffusion steps | All layers | ||
|---|---|---|---|---|---|
| Latte | t2v | 50 | 12 | 28 | 21 |
| Latte | c2v | 250 | 60 | 14 | 10 |
| Hunyuan | t2i | 50 | 12 | 28 | 18 |
| DiT-XL/2 | c2i | 250 | 60 | 28 | 21 |
PAB
PAB (Pyramid Attention Broadcast) [47] is one of the most promising caching methods designed for real-time video generation. The method leverages the observation that attention differences during the diffusion process follow a U-shaped pattern, broadcasting attention outputs to subsequent steps in a pyramid-like manner. Different broadcast ranges are set for three types of attention—spatial, temporal, and cross-attention—based on their respective differences. denotes the broadcast ranges for spatial (), temporal (), and cross () attentions.
In this work, we use the official implementation of PAB for text-to-video tasks on Latte and adapt the caching method to other tasks in-house. For the class-to-video task, where cross-attention is absent, refers to the broadcast ranges of spatial () and temporal () attentions. In the text-to-image task, which lacks temporal attention, instead denotes the broadcast ranges of spatial () and cross () attentions. We do not apply PAB to the class-to-image task, as it involves only spatial attention.
| T-GATE | Task | Diffusion steps | m | k |
|---|---|---|---|---|
| Latte | t2v | 50 | 20 | 2 |
| Hunyuan-DiT | t2i | 50 | 20 | 2 |
T-Gates
T-Gates divide the diffusion process into two phases: (1) the Semantics-Planning Phase and (2) the Fidelity-Improving Phase. In the first phase, self-attention is computed and reused every steps. In the second phase, cross-attention is cached using a caching mechanism. The hyperparameter determines the boundary between these two phases. For our implementation, we use the same hyperparameters as PAB [47]. Detailed configurations are provided in Table 10.
FORA
FORA (Fast-Forward Caching) [30] stores and reuses intermediate outputs from attention and MLP layers across denoising steps. However, in the original FORA paper, features are cached in advance before the diffusion process. We do not adopt this approach, as it is a highly time-consuming process. Instead, in this work, we skip the “Initialization” step in FORA and calculate the features dynamically during the diffusion process.
9 Case Study
Video Generation
In Figure 5, we showcase the generated video frames from text prompts with Skip-Cache , PAB, and comparing them to the original model. From generating portraits to scenery, Skip-Cache consistently demonstrates better visual fidelity along with faster generation speeds. Figure 6 presents class-to-video generation examples with Skip-Cache with varying caching steps . By comparing Skip-Cache to Original model, we see Skip-Cache maintain good generation quality across different caching steps.
Image Generation
Figure 7 compares qualitative results of Skip-Cache compared to other caching-based acceleration methods (-DiT, FORA, T-GATE) on Hunyuan-DiT. In Figure 8, Skip-Cache show distinct edges in higher speedup and similarity to the original generation, while other baselines exist with different degrees of change in details such as color, texture, and posture. Similarly, we present Skip-Cache with varying caching steps in Figure 8, showing that with more steps cached, it still maintains high fidelity to the original generation.

---

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

---

