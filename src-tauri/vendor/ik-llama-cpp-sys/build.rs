//! Build script for `ik-llama-cpp-sys` (SOULS MC: Windows MSVC port patch).
//!
//! Original upstream: Linux (CPU + CUDA) focused for v1. This patch adds
//! full Windows MSVC support via [`parse_target_os`], matching the gabarito
//! of `utilityai/llama-cpp-sys-2` build.rs (Marco IV / ADR-030 Canibalização).
//!
//! Two build modes:
//!   * **Prebuilt fast-path** — if `IK_LLAMA_CPP_LIB_DIR` is set, skip CMake and
//!     link the prebuilt `libllama`/`libggml` (+ static `libcommon.a`
//!     under `common`) found there. Bindgen still runs off the source headers.
//!   * **CMake build** — otherwise build ik_llama.cpp from source (submodule or
//!     `IK_LLAMA_CPP_SRC`) with `-DGGML_MAX_CONTEXTS=2048`.
//!
//! ik has no `install()` rules for the static archives and no
//! `ggml-config.cmake` / `LLAMA_USE_SYSTEM_GGML` — so this crate's build.rs is
//! the single source of link truth (prebuilt layout is our convention).

use std::env;
use std::path::{Path, PathBuf};

// ============================================================================
// SOULS MC: Windows MSVC detection (Marco IV / ADR-030)
// ============================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum WindowsVariant {
    Msvc,
    Other,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum TargetOs {
    Windows(WindowsVariant),
    Apple,
    Linux,
    Android,
    Other,
}

fn parse_target_os() -> (TargetOs, &'static str) {
    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("windows") {
        if target.ends_with("-windows-msvc") {
            (TargetOs::Windows(WindowsVariant::Msvc), "windows-msvc")
        } else {
            (TargetOs::Windows(WindowsVariant::Other), "windows-gnu")
        }
    } else if target.contains("apple") || target.contains("darwin") {
        (TargetOs::Apple, "apple")
    } else if target.contains("android") {
        (TargetOs::Android, "android")
    } else if target.contains("linux") {
        (TargetOs::Linux, "linux")
    } else {
        (TargetOs::Other, "other")
    }
}

fn is_windows_msvc() -> bool {
    parse_target_os().0 == TargetOs::Windows(WindowsVariant::Msvc)
}

fn feat(name: &str) -> bool {
    env::var(format!("CARGO_FEATURE_{name}")).is_ok()
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Allow the source to live somewhere else (e.g. a workspace-level
    // `vendor/ik-llama-cpp-sys/ik_llama.cpp/`). Default: alongside the crate.
    let src = env::var("IK_LLAMA_CPP_SRC")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir.join("ik_llama.cpp"));
    assert!(
        src.join("include/llama.h").exists(),
        "ik_llama.cpp source not found at {src:?} — set IK_LLAMA_CPP_SRC or run `git submodule update --init`"
    );

    let want_common = feat("COMMON");
    let want_mtmd = feat("MTMD");
    let want_cuda = feat("CUDA");
    let want_vulkan = feat("VULKAN");
    let want_metal = feat("METAL") && cfg!(target_os = "macos");
    let want_openmp = feat("OPENMP");
    let want_native = feat("NATIVE");
    let dynamic_link = feat("DYNAMIC_LINK");
    let static_stdcxx = feat("STATIC_STDCXX");
    let win_msvc = is_windows_msvc();

    if want_cuda && !win_msvc {
        // Keep NVCC out of the way on non-MSVC: the original Linux focus.
        eprintln!("ik-llama-cpp-sys: CUDA requested but target is not Windows MSVC; CMake will handle per-OS.");
    }

    for f in [
        "wrapper.h",
        "wrapper_common.h",
        "wrapper_common.cpp",
        "wrapper_grammar.h",
        "wrapper_grammar.cpp",
        "wrapper_utils.h",
        "wrapper_mtmd.h",
        "build.rs",
    ] {
        println!("cargo:rerun-if-changed={f}");
    }
    println!("cargo:rerun-if-changed=ik_llama.cpp/common");
    println!("cargo:rerun-if-changed=ik_llama.cpp/src");
    println!("cargo:rerun-if-changed=ik_llama.cpp/ggml/src");
    println!("cargo:rerun-if-changed=ik_llama.cpp/ggml/include");
    println!("cargo:rerun-if-changed=ik_llama.cpp/include");
    println!("cargo:rerun-if-env-changed=IK_LLAMA_CPP_SRC");
    println!("cargo:rerun-if-env-changed=IK_LLAMA_CPP_LIB_DIR");

    // ---- bindgen (both modes) ----
    generate_bindings(&src, &out_dir, want_common);

    if want_mtmd {
        generate_mtmd_bindings(&src, &out_dir);
    }

    if env::var("DOCS_RS").is_ok() {
        return;
    }

    compile_grammar_glue(&src, &manifest_dir);
    if want_common {
        compile_common_glue(&src, &manifest_dir);
    }
    if want_mtmd {
        compile_mtmd(&src);
    }

    let backend = if let Some(lib_dir) = env::var("IK_LLAMA_CPP_LIB_DIR").ok().map(PathBuf::from) {
        link_prebuilt(&lib_dir, want_common, want_cuda);
        format!("prebuilt:{}", lib_dir.display())
    } else {
        let dst = cmake_build(
            &src,
            want_common,
            want_cuda,
            want_vulkan,
            want_metal,
            want_openmp,
            want_native,
            dynamic_link,
        );
        link_built(&dst, want_common, dynamic_link, win_msvc);
        "cmake".to_string()
    };

    if want_cuda {
        link_cuda(win_msvc);
    }

    if want_vulkan {
        link_vulkan();
    }

    if cfg!(target_os = "macos") {
        link_apple_cpu();
    }

    if want_metal {
        link_metal();
    }

    // ---- C++ stdlib + system libs ----
    // SOULS MC: Windows MSVC treats CRT, m, pthread, dl natively — never emit
    // any of the GNU `-l*` flags. cl.exe / link.exe link against the C++ MSVC
    // runtime implicitly when the cc-rs glue is compiled with `/MT` or `/MD`.
    link_system(static_stdcxx, want_openmp, win_msvc);

    write_manifest(
        &src,
        &out_dir,
        want_cuda,
        want_vulkan,
        want_metal,
        want_openmp,
        want_common,
        &backend,
    );
}

fn generate_bindings(src: &Path, out_dir: &Path, want_common: bool) {
    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", src.join("include").display()))
        .clang_arg(format!("-I{}", src.join("ggml/include").display()))
        .allowlist_function("ggml_.*")
        .allowlist_type("ggml_.*")
        .allowlist_var("ggml_.*")
        .allowlist_function("gguf_.*")
        .allowlist_type("gguf_.*")
        .allowlist_var("gguf_.*")
        .allowlist_function("llama_.*")
        .allowlist_type("llama_.*")
        .allowlist_var("llama_.*")
        .allowlist_function("ik_llama_rs_grammar_.*")
        .prepend_enum_name(false)
        .derive_partialeq(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    if want_common {
        builder = builder
            .clang_arg("-DLLAMA_RS_BUILD_COMMON")
            .clang_arg(format!("-I{}", src.join("common").display()))
            .allowlist_function("ik_llama_rs_.*")
            .allowlist_type("ik_llama_rs_.*");
    }

    builder
        .generate()
        .expect("bindgen failed to generate ik_llama.cpp bindings")
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("failed to write bindings.rs");
}

fn generate_mtmd_bindings(src: &Path, out_dir: &Path) {
    let mtmd_dir = src.join("examples/mtmd");
    assert!(
        mtmd_dir.join("mtmd.h").exists(),
        "mtmd feature set but mtmd.h not found at {mtmd_dir:?}"
    );

    let builder = bindgen::Builder::default()
        .header(mtmd_dir.join("mtmd.h").display().to_string())
        .clang_arg("-x")
        .clang_arg("c++")
        .clang_arg("-std=c++17")
        .clang_arg(format!("-I{}", src.join("include").display()))
        .clang_arg(format!("-I{}", src.join("ggml/include").display()))
        .clang_arg(format!("-I{}", src.join("vendor").display()))
        .clang_arg(format!("-I{}", mtmd_dir.display()))
        .allowlist_function("mtmd_.*")
        .allowlist_type("mtmd_.*")
        .allowlist_var("mtmd_.*")
        // Blocklist the two functions that take a C++ `json&` (nlohmann/json.hpp).
        .blocklist_function("mtmd_input_chunk_from_json")
        .blocklist_function("mtmd_input_chunk_to_json")
        .prepend_enum_name(false)
        .derive_partialeq(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    builder
        .generate()
        .expect("bindgen failed to generate mtmd bindings")
        .write_to_file(out_dir.join("mtmd_bindings.rs"))
        .expect("failed to write mtmd_bindings.rs");
}

fn dirs_with(root: &Path, pred: &dyn Fn(&str) -> bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root).into_iter().flatten() {
        if entry.file_type().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if pred(name) {
                    if let Some(parent) = entry.path().parent() {
                        let p = parent.to_path_buf();
                        if !out.contains(&p) {
                            out.push(p);
                        }
                    }
                }
            }
        }
    }
    out
}

/// Prebuilt fast-path: link the shared `libllama`/`libggml` (+ static `libcommon.a`
/// under `common`) found under `lib_dir`.
///
/// **SOULS MC Windows MSVC**: on MSVC, files are named `llama.lib` / `ggml.lib`
/// (no `lib` prefix, `.lib` extension). GNU/Linux uses `libllama.so` / `libggml.so`.
fn link_prebuilt(lib_dir: &Path, want_common: bool, _want_cuda: bool) {
    assert!(
        lib_dir.exists(),
        "IK_LLAMA_CPP_LIB_DIR does not exist: {lib_dir:?}"
    );

    let win_msvc = is_windows_msvc();
    let so_dirs = if win_msvc {
        // Windows MSVC: bare names, no `lib` prefix, `.lib` extension.
        dirs_with(lib_dir, &|n| {
            n == "llama.lib" || (n.starts_with("ggml") && n.ends_with(".lib"))
        })
    } else {
        // GNU/Linux: `lib` prefix, `.so` extension.
        dirs_with(lib_dir, &|n| {
            n.starts_with("libllama.so") || n.starts_with("libggml") && n.contains(".so")
        })
    };
    assert!(
        !so_dirs.is_empty(),
        "no llama.lib/ggml*.lib (MSVC) or libllama.so/libggml*.so (GNU) found under {lib_dir:?}"
    );
    for d in &so_dirs {
        println!("cargo:rustc-link-search=native={}", d.display());
        if !win_msvc {
            // `-Wl,-rpath` is GNU-only. MSVC uses PATH for DLL resolution.
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", d.display());
        }
    }

    if want_common {
        let common_dirs = if win_msvc {
            dirs_with(lib_dir, &|n| n == "common.lib")
        } else {
            dirs_with(lib_dir, &|n| n == "libcommon.a")
        };
        assert!(
            !common_dirs.is_empty(),
            "`common` feature set but no common.lib (MSVC) or libcommon.a (GNU) found under {lib_dir:?}"
        );
        for d in &common_dirs {
            println!("cargo:rustc-link-search=native={}", d.display());
        }
        println!("cargo:rustc-link-lib=static=common");
    }

    // ik ggml is monolithic. On Windows MSVC link the .lib; on GNU link the .so.
    println!("cargo:rustc-link-lib=dylib=llama");
    println!("cargo:rustc-link-lib=dylib=ggml");
}

#[allow(clippy::too_many_arguments)]
fn cmake_build(
    src: &Path,
    _want_common: bool,
    want_cuda: bool,
    want_vulkan: bool,
    want_metal: bool,
    want_openmp: bool,
    want_native: bool,
    dynamic_link: bool,
) -> PathBuf {
    let mut cfg = cmake::Config::new(src);
    cfg.define("GGML_MAX_CONTEXTS", "2048")
        .define("LLAMA_CURL", "OFF")
        .define("LLAMA_BUILD_TESTS", "OFF")
        .define("LLAMA_BUILD_EXAMPLES", "OFF")
        .define("LLAMA_BUILD_SERVER", "OFF")
        .define("BUILD_SHARED_LIBS", if dynamic_link { "ON" } else { "OFF" })
        .define("GGML_NATIVE", if want_native { "ON" } else { "OFF" })
        .define("GGML_OPENMP", if want_openmp { "ON" } else { "OFF" });
    if want_cuda {
        cfg.define("GGML_CUDA", "ON");
        cfg.define("GGML_NCCL", "OFF");
    }
    if want_vulkan {
        cfg.define("GGML_VULKAN", "ON");
    }
    if cfg!(target_os = "macos") {
        cfg.define("GGML_METAL", if want_metal { "ON" } else { "OFF" });
        if want_metal {
            cfg.define("GGML_METAL_EMBED_LIBRARY", "ON");
        }
    }
    cfg.build()
}

/// Link the archives/libs from a from-source CMake build.
///
/// **SOULS MC Windows MSVC (Marco IV)**:
///   * MSVC Ninja generator places static archives in `dst/lib/` (flat), not
///     `dst/build/` (the Linux multi-config layout). On MSVC we look in `dst/lib/`.
///   * MSVC archives are `llama.lib` / `ggml.lib` / `common.lib` (no `lib` prefix,
///     `.lib` extension). GNU/Linux uses `libllama.a` / `libggml.a` / `libcommon.a`.
fn link_built(dst: &Path, want_common: bool, dynamic_link: bool, win_msvc: bool) {
    // On Windows MSVC + Ninja, archives live under `dst/lib/`. On GNU multi-config
    // (Make/MSBuild), they live under `dst/build/`. Try `build/` first, then `lib/`.
    let candidates = [dst.join("build"), dst.join("lib"), dst.to_path_buf()];
    let search_root = candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| dst.to_path_buf());

    let kind = if dynamic_link { "dylib" } else { "static" };

    // SOULS MC: extension filter is OS-specific.
    //   * GNU/Linux static: `.a`; dynamic: contains `.so`
    //   * Windows MSVC: `.lib` (regardless of static/dynamic, MSVC uses .lib for both)
    let ext_ok = |n: &str| {
        if win_msvc {
            n.ends_with(".lib")
        } else if dynamic_link {
            n.contains(".so")
        } else {
            n.ends_with(".a")
        }
    };

    // SOULS MC: file prefix is OS-specific.
    //   * GNU/Linux: `libllama.a`, `libggml.a`, `libcommon.a`
    //   * Windows MSVC: `llama.lib`, `ggml.lib`, `common.lib`
    let (llama_prefix, ggml_prefix, common_prefix) = if win_msvc {
        ("llama", "ggml", "common")
    } else {
        ("libllama", "libggml", "libcommon")
    };

    let mut wanted: Vec<(&str, &str)> = Vec::new();
    if want_common {
        wanted.push(("common", common_prefix));
    }
    wanted.push(("llama", llama_prefix));
    wanted.push(("ggml", ggml_prefix));

    for (link_name, file_prefix) in wanted {
        let dirs = dirs_with(&search_root, &|n| n.starts_with(file_prefix) && ext_ok(n));
        assert!(
            !dirs.is_empty(),
            "could not find {file_prefix}.* under {search_root:?} after CMake build"
        );
        for d in &dirs {
            println!("cargo:rustc-link-search=native={}", d.display());
            if dynamic_link && !win_msvc {
                println!("cargo:rustc-link-arg=-Wl,-rpath,{}", d.display());
            }
        }
        println!("cargo:rustc-link-lib={kind}={link_name}");
    }
}

fn compile_grammar_glue(src: &Path, manifest_dir: &Path) {
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .file(manifest_dir.join("wrapper_grammar.cpp"))
        .include(manifest_dir)
        .include(src.join("include"))
        .include(src.join("ggml/include"))
        .include(src.join("src"))
        .flag_if_supported("-fPIC")
        .warnings(false);
    build.compile("ik_llama_rs_grammar");
}

fn compile_common_glue(src: &Path, manifest_dir: &Path) {
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .file(manifest_dir.join("wrapper_common.cpp"))
        .include(manifest_dir)
        .include(src.join("include"))
        .include(src.join("ggml/include"))
        .include(src.join("common"))
        .include(src.join("src"))
        .include(src.join("vendor"))
        .define("LLAMA_RS_BUILD_COMMON", None)
        .flag_if_supported("-fPIC")
        .warnings(false);
    build.compile("ik_llama_rs_common");
}

fn compile_mtmd(src: &Path) {
    let mtmd_dir = src.join("examples/mtmd");
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .file(mtmd_dir.join("mtmd.cpp"))
        .file(mtmd_dir.join("mtmd-audio.cpp"))
        .file(mtmd_dir.join("clip.cpp"))
        .file(mtmd_dir.join("mtmd-helper.cpp"))
        .include(&mtmd_dir)
        .include(src)
        .include(src.join("include"))
        .include(src.join("ggml/include"))
        .include(src.join("vendor"))
        .flag_if_supported("-Wno-cast-qual")
        .pic(true)
        .warnings(false);
    build.compile("mtmd");
}

/// Link CUDA runtime. **SOULS MC**: on Windows MSVC, libraries are `cudart.lib`
/// etc. (and the import library for `cuda.dll` is `cuda.lib`); on GNU/Linux the
/// sonames are bare (`cudart`, `cublas`, `cuda`).
fn link_cuda(win_msvc: bool) {
    // Look for CUDA toolkit: on Windows honour `CUDA_PATH` (e.g. C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.3);
    // on Linux honour `CUDA_PATH` too plus the well-known /opt/cuda and /usr/local/cuda.
    let candidates: Vec<PathBuf> = if win_msvc {
        env::var("CUDA_PATH")
            .ok()
            .map(PathBuf::from)
            .into_iter()
            .chain(
                [
                    r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.3",
                    r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.8",
                ]
                .iter()
                .map(PathBuf::from),
            )
            .collect()
    } else {
        env::var("CUDA_PATH")
            .ok()
            .map(PathBuf::from)
            .into_iter()
            .chain(
                ["/opt/cuda", "/usr/local/cuda"]
                    .iter()
                    .map(PathBuf::from),
            )
            .collect()
    };

    for c in &candidates {
        // Windows uses `lib\x64`; GNU uses `lib64`; Mac uses `lib`.
        let lib_subdirs: &[&str] = if win_msvc { &["lib/x64", "lib"] } else { &["lib64", "lib"] };
        for sub in lib_subdirs {
            let lib_dir = c.join(sub);
            if lib_dir.exists() {
                println!("cargo:rustc-link-search=native={}", lib_dir.display());
                if win_msvc {
                    // The driver-API stub library on Windows lives in `lib/x64/cuda.lib` (import
                    // lib for `nvcuda.dll`). Real driver resolves at runtime.
                    let stubs = lib_dir.join("stubs");
                    if stubs.exists() {
                        println!("cargo:rustc-link-search=native={}", stubs.display());
                    }
                }
            }
        }
    }

    // Emit link-lib with the correct extension per OS.
    let ext = if win_msvc { ".lib" } else { "" };
    println!("cargo:rustc-link-lib=dylib=cudart{ext}");
    println!("cargo:rustc-link-lib=dylib=cublas{ext}");
    println!("cargo:rustc-link-lib=dylib=cuda{ext}");
}

fn link_vulkan() {
    if let Ok(sdk) = env::var("VULKAN_SDK") {
        let lib = PathBuf::from(&sdk).join(if cfg!(target_os = "windows") {
            "Lib"
        } else {
            "lib"
        });
        if lib.exists() {
            println!("cargo:rustc-link-search=native={}", lib.display());
        }
    }
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=dylib=vulkan-1");
    } else {
        println!("cargo:rustc-link-lib=dylib=vulkan");
    }
}

fn link_apple_cpu() {
    println!("cargo:rustc-link-lib=framework=Accelerate");
}

fn link_metal() {
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=Metal");
    println!("cargo:rustc-link-lib=framework=MetalKit");
}

/// Link the C++ stdlib + POSIX-y system libs.
///
/// **SOULS MC (Marco IV)**: on Windows MSVC, `cl.exe`/`link.exe` handle the CRT
/// (`vcruntime`, `msvcrt`, `ucrt`) and `kernel32` implicitly; `-lstdc++`, `-ldl`,
/// `-lm`, `-lpthread` are GNU-only and would fail with `unresolved external`.
/// We early-return for MSVC and emit nothing.
/// On macOS, libc++ (not libstdc++) and libSystem cover everything.
/// On GNU/Linux, emit the GNU links.
fn link_system(static_stdcxx: bool, want_openmp: bool, win_msvc: bool) {
    // SOULS MC: Windows MSVC: cl.exe/link.exe treat the CRT natively. No `-l*` flags.
    if win_msvc {
        return;
    }

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
        return;
    }

    // GNU/Linux (and other GNU targets).
    if static_stdcxx {
        println!("cargo:rustc-link-lib=static=stdc++");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
    if want_openmp {
        println!("cargo:rustc-link-lib=dylib=gomp");
    }
    println!("cargo:rustc-link-lib=dylib=m");
    println!("cargo:rustc-link-lib=dylib=pthread");
    println!("cargo:rustc-link-lib=dylib=dl");
}

#[allow(clippy::too_many_arguments)]
fn write_manifest(
    src: &Path,
    out_dir: &Path,
    want_cuda: bool,
    want_vulkan: bool,
    want_metal: bool,
    want_openmp: bool,
    want_common: bool,
    backend: &str,
) {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(512);
    let _ = writeln!(
        s,
        "ik-llama-cpp-sys build manifest\n  backend: {backend}\n  src: {}",
        src.display()
    );
    let _ = writeln!(s, "  cuda={want_cuda} vulkan={want_vulkan} metal={want_metal} openmp={want_openmp} common={want_common}");
    std::fs::write(out_dir.join("ik_llama_cpp_sys_build_manifest.txt"), s).ok();
}
