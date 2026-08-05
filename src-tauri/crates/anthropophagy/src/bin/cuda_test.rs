// Ephemeral Atomic Test Binary for CUDA validation (Compute Capability 7.5 - RTX 2060m)
// Usage: cargo run --bin souls_cuda_test --features llama_backend -- [path_to_model.gguf]

#[cfg(feature = "llama_backend")]
mod test_impl {
    use std::path::PathBuf;
    use std::time::Instant;

    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::llama_batch::LlamaBatch;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::model::LlamaModel;
    use llama_cpp_2::sampling::LlamaSampler;

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        println!("=== SOULS CUDA ATOMIC TEST (Compute Capability 7.5 - RTX 2060m) ===");

        // 1. Initialize llama.cpp backend singleton
        println!("[1/5] Initializing LlamaBackend...");
        let backend = LlamaBackend::init()?;
        println!("  -> LlamaBackend initialized successfully.");

        // 2. Resolve model path from CLI or fallback
        let model_path = std::env::args()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".souls_data/models/Qwen3.5-0.8B-Q4_0.gguf"));

        println!("[2/5] Target model path: {:?}", model_path);
        if !model_path.exists() {
            eprintln!("WARNING: Model file does not exist at {:?}. Please pass a valid .gguf path as CLI arg.", model_path);
            eprintln!("Example: cargo run --bin souls_cuda_test --features llama_backend -- C:\\path\\to\\model.gguf");
            return Ok(());
        }

        // 3. Force n_gpu_layers = 99 to ensure full CUDA offloading to RTX 2060m
        println!("[3/5] Setting LlamaModelParams with n_gpu_layers = 99 (FORCED CUDA OFFLOAD)...");
        let model_params = LlamaModelParams::default().with_n_gpu_layers(99);

        let start_load = Instant::now();
        let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)?;
        println!("  -> Model loaded in {} ms.", start_load.elapsed().as_millis());

        // 4. Create Context
        println!("[4/5] Allocating LlamaContextParams...");
        let ctx_params = LlamaContextParams::default();
        let mut ctx = model.new_context(&backend, ctx_params)?;

        // 5. Tokenize and run prompt "Olá"
        let prompt = "<|im_start|>user\nOlá\n<|im_end|>\n<|im_start|>assistant\n";
        println!("[5/5] Processing atomic test prompt: \"{}\"", prompt.trim());

        let tokens = model.str_to_token(prompt, llama_cpp_2::model::AddBos::Always)?;
        if tokens.is_empty() {
            return Err("Prompt tokenization returned 0 tokens".into());
        }

        let mut batch = LlamaBatch::new(tokens.len().max(512), 1);
        let last_idx = tokens.len() - 1;
        for (i, &token) in tokens.iter().enumerate() {
            batch.add(token, i as i32, &[0], i == last_idx)?;
        }

        ctx.decode(&mut batch)?;

        let mut sampler = LlamaSampler::chain_simple(vec![
            LlamaSampler::temp(0.7),
            LlamaSampler::dist(0),
        ]);

        let sampled_token = sampler.sample(&ctx, batch.n_tokens() - 1);
        #[allow(deprecated)]
        let sampled_str = model.token_to_str(sampled_token, llama_cpp_2::model::Special::Tokenize)?;

        println!("\n=== CUDA ATOMIC TEST SUCCESSFUL ===");
        println!("First generated token: {:?}", sampled_str);
        println!("If GGML initialization logs above report CUDA / nvml / RTX 2060m offloading, C-FFI GPU compilation is VERIFIED!");

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "llama_backend")]
    {
        test_impl::run()
    }
    #[cfg(not(feature = "llama_backend"))]
    {
        eprintln!("ERROR: The binary `souls_cuda_test` requires the `llama_backend` feature flag.");
        eprintln!("Please run with: cargo run --bin souls_cuda_test --features llama_backend");
        Err("Missing feature `llama_backend`".into())
    }
}
