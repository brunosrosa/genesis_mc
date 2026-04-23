# 🧠 SODA CANONICAL KNOWLEDGE BASE
> Gerado pelo @soda-knowledge-curator.
> Fontes Originais: 249 | Fontes Puras: 247 | Temas Identificados: 24



## 🧩 Eixo Temático 7

# GitHub - docling-project/docling: Get your documents ready for gen AI · GitHub
Source URL: https://github.com/docling-project/docling

Source Type: web_page

Source ID: 59560af0-e765-4972-8e7d-ce41c9708a9a


Docling simplifies document processing, parsing diverse formats — including advanced PDF understanding — and providing seamless integrations with the gen AI ecosystem.
- 🗂️ Parsing of multiple document formats incl. PDF, DOCX, PPTX, XLSX, HTML, WAV, MP3, WebVTT, images (PNG, TIFF, JPEG, ...), LaTeX, plain text, and more
- 📑 Advanced PDF understanding incl. page layout, reading order, table structure, code, formulas, image classification, and more
- 🧬 Unified, expressive DoclingDocument representation format
- ↪️ Various export formats and options, including Markdown, HTML, WebVTT, DocTags and lossless JSON
- 📜 Support of several application-specifc XML schemas incl. USPTO patents, JATS articles, and XBRL financial reports.
- 🔒 Local execution capabilities for sensitive data and air-gapped environments
- 🤖 Plug-and-play integrations incl. LangChain, LlamaIndex, Crew AI & Haystack for agentic AI
- 🔍 Extensive OCR support for scanned PDFs and images
- 👓 Support of several Visual Language Models (GraniteDocling)
- 🎙️ Audio support with Automatic Speech Recognition (ASR) models
- 🔌 Connect to any agent using the MCP server
- 💻 Simple and convenient CLI
- 📤 Structured information extraction [🧪 beta]
- 📑 New layout model (Heron) by default, for faster PDF parsing
- 🔌 MCP server for agentic applications
- 💼 Parsing of XBRL (eXtensible Business Reporting Language) documents for financial reports
- 💬 Parsing of WebVTT (Web Video Text Tracks) files and export to WebVTT format
- 💬 Parsing of LaTeX files
- 📝 Parsing of plain-text files (
.txt
,.text
) and Markdown supersets (.qmd
,.Rmd
) - 📝 Chart understanding (Barchart, Piechart, LinePlot): converting them into tables, code or adding detailed descriptions
- 📝 Metadata extraction, including title, authors, references & language
- 📝 Complex chemistry understanding (Molecular structures)
To use Docling, simply install docling
from your package manager, e.g. pip:
pip install docling
Note: Python 3.9 support was dropped in docling version 2.70.0. Please use Python 3.10 or higher.
Works on macOS, Linux and Windows environments. Both x86_64 and arm64 architectures.
More detailed installation instructions are available in the docs.
To convert individual documents with python, use convert()
, for example:
from docling.document_converter import DocumentConverter
source = "https://arxiv.org/pdf/2408.09869" # document per local path or URL
converter = DocumentConverter()
result = converter.convert(source)
print(result.document.export_to_markdown()) # output: "## Docling Technical Report[...]"
More advanced usage options are available in the docs.
Docling has a built-in CLI to run conversions.
docling https://arxiv.org/pdf/2206.01062
You can also use 🥚GraniteDocling and other VLMs via Docling CLI:
docling --pipeline vlm --vlm-model granite_docling https://arxiv.org/pdf/2206.01062
This will use MLX acceleration on supported Apple Silicon hardware.
Read more here
Check out Docling's documentation, for details on installation, usage, concepts, recipes, extensions, and more.
Go hands-on with our examples, demonstrating how to address different application use cases with Docling.
To further accelerate your AI application development, check out Docling's native integrations with popular frameworks and tools.
Please feel free to connect with us using the discussion section.
For more details on Docling's inner workings, check out the Docling Technical Report.
Please read Contributing to Docling for details.
If you use Docling in your projects, please consider citing the following:
@techreport{Docling,
author = {Deep Search Team},
month = {8},
title = {Docling Technical Report},
url = {https://arxiv.org/abs/2408.09869},
eprint = {2408.09869},
doi = {10.48550/arXiv.2408.09869},
version = {1.0.0},
year = {2024}
}
The Docling codebase is under MIT license. For individual model usage, please refer to the model licenses found in the original packages.
Docling is hosted as a project in the LF AI & Data Foundation.
The project was started by the AI for knowledge team at IBM Research Zurich.

---

# kreuzberg 4.4.6 - Docs.rs
Source URL: https://docs.rs/crate/kreuzberg/latest

Source Type: web_page

Source ID: a7033a15-517b-468a-8597-c59c3094a245


Kreuzberg
High-performance document intelligence library for Rust. Extract text, metadata, and structured information from PDFs, Office documents, images, and 75 formats.
This is the core Rust library that powers the Python, TypeScript, and Ruby bindings.
🚀 Version 4.9.4 Release This is a pre-release version. We invite you to test the library and report any issues you encounter.
Note: The Rust crate is not currently published to crates.io for this RC. Use git dependencies or language bindings (Python, TypeScript, Ruby) instead.
Installation
[]
= "4.0"
= { = "1", = ["rt", "macros"] }
PDFium Linking Options
Kreuzberg offers flexible PDFium linking strategies for different deployment scenarios. Note: Language bindings (Python, TypeScript, Ruby, Java, Go, C#, PHP, Elixir) automatically bundle PDFium—no configuration needed. This section applies only to the Rust crate.
| Strategy | Feature Flag | Description | Use Case |
|---|---|---|---|
| Default (Dynamic) | None | Links to system PDFium at runtime | Development, system package users |
| Static | pdf-static |
Statically links PDFium into binary | Single binary distribution, no runtime dependencies |
| Bundled | pdf-bundled |
Downloads and embeds PDFium in binary | CI/CD, hermetic builds, largest binary size |
| System | pdf-system |
Uses system PDFium via pkg-config | Linux distributions with PDFium package |
Example Cargo.toml configurations:
# Default (dynamic linking)
[]
= "4.0"
# Static linking
[]
= { = "4.0", = ["pdf-static"] }
# Bundled in binary
[]
= { = "4.0", = ["pdf-bundled"] }
# System library (requires PDFium installed)
[]
= { = "4.0", = ["pdf-system"] }
For more details on feature flags and configuration options, see the Features documentation.
System Requirements
ONNX Runtime (for embeddings)
If using embeddings functionality, ONNX Runtime must be installed:
# macOS
# Ubuntu/Debian
# Windows (MSVC)
# OR download from https://github.com/microsoft/onnxruntime/releases
Without ONNX Runtime, embeddings will raise MissingDependencyError
with installation instructions.
Quick Start
use ;
Async Extraction
use ;
async
Batch Processing
use ;
async
OCR with Table Extraction
use ;
Password-Protected PDFs
use ;
Extract from Bytes
use ;
use fs;
Code Intelligence
Kreuzberg integrates tree-sitter-language-pack to parse and analyze source code files across 248 programming languages. When you extract a source code file, Kreuzberg automatically detects the language and produces structured analysis including functions, classes, imports, exports, symbols, diagnostics, and semantic code chunks.
Code intelligence data is available via the metadata.format
field as a FormatMetadata::Code
variant containing a ProcessResult
.
use ;
Requires the tree-sitter
feature flag (included in full
). See the Code Intelligence Guide for configuration details and examples in all languages.
Features
The crate uses feature flags for optional functionality:
[]
= { = "4.0", = ["pdf", "excel", "ocr"] }
Available Features
| Feature | Description | Binary Size |
|---|---|---|
pdf |
PDF extraction via pdfium | +25MB |
excel |
Excel/spreadsheet parsing | +3MB |
office |
DOCX, PPTX extraction | +1MB |
email |
EML, MSG extraction | +500KB |
html |
HTML to markdown | +1MB |
xml |
XML streaming parser | +500KB |
archives |
ZIP, TAR, 7Z extraction | +2MB |
ocr |
OCR with Tesseract | +5MB |
language-detection |
Language detection | +100KB |
chunking |
Text chunking | +200KB |
quality |
Text quality processing | +500KB |
Feature Bundles
= { = "4.0", = ["full"] }
= { = "4.0", = ["server"] }
= { = "4.0", = ["cli"] }
PDF Support and Linking Options
Kreuzberg supports three PDFium linking strategies. Default is bundled-pdfium
(best developer experience).
| Strategy | Feature | Use Case | Binary Size | Runtime Deps |
|---|---|---|---|---|
| Bundled (default) | bundled-pdfium |
Development, production | +8-15MB | None |
| Static | static-pdfium |
Docker, musl, standalone binaries | +200MB | None |
| System | system-pdfium |
Package managers, distros | +2MB | libpdfium.so |
Quick Start
# Default - bundled PDFium (recommended)
[]
= "4.0"
# Static linking (Docker, musl)
[]
= { = "4.0", = ["static-pdfium"] }
# System PDFium (package managers)
[]
= { = "4.0", = ["system-pdfium"] }
For detailed information, see the PDFium Linking Guide in the project documentation.
Note: Language bindings (Python, TypeScript, Ruby, Java, Go) automatically bundle PDFium. No configuration needed.
Documentation
API Documentation – Complete API reference with examples
https://docs.kreuzberg.dev – User guide and tutorials
License
Elastic License 2.0 (ELv2) - see LICENSE for details.

---

# kreuzberg-dev/kreuzberg: A polyglot document intelligence ... - GitHub
Source URL: https://github.com/kreuzberg-dev/kreuzberg

Source Type: web_page

Source ID: e7eadb0a-40c6-4a9a-ae68-ba35f71a981a


Extract text, metadata, and code intelligence from 97+ file formats and 305 programming languages at native speeds without needing a GPU.
- Code intelligence – Extract functions, classes, imports, symbols, and docstrings from 248 programming languages via tree-sitter. Results in
ExtractionResult.code_intelligence
with semantic chunking - Extensible architecture – Plugin system for custom OCR backends, validators, post-processors, document extractors, and renderers
- Polyglot – Native bindings for Rust, Python, TypeScript/Node.js, Ruby, Go, Java, C#, PHP, Elixir, R, and C
- 91+ file formats – PDF, Office documents, images, HTML, XML, emails, archives, academic formats across 8 categories
- LLM intelligence – VLM OCR (GPT-4o, Claude, Gemini, Ollama), structured JSON extraction with schema constraints, and provider-hosted embeddings via 146 LLM providers (including local engines: Ollama, LM Studio, vLLM, llama.cpp) through liter-llm
- OCR support – Tesseract (all bindings, including Tesseract-WASM for browsers), PaddleOCR (all native bindings), EasyOCR (Python), VLM OCR (146 vision model providers including local engines), extensible via plugin API
- High performance – Rust core with native PDFium, SIMD optimizations and full parallelism
- Flexible deployment – Use as library, CLI tool, REST API server, or MCP server
- TOON wire format – Token-efficient serialization for LLM/RAG pipelines, ~30-50% fewer tokens than JSON
- GFM-quality output – Comrak-based rendering with proper fenced code blocks, table nodes, bracket escaping, and cross-format parity (Markdown, HTML, Djot, Plain)
- HTML passthrough – HTML-to-Markdown conversion uses html-to-markdown output directly, bypassing lossy intermediate round-trips
- Memory efficient – Streaming parsers for multi-GB files
Complete Documentation | Live Demo | Installation Guides
Each language binding provides comprehensive documentation with examples and best practices. Choose your platform to get started:
Scripting Languages:
- Python – PyPI package, async/sync APIs, OCR backends (Tesseract, PaddleOCR, EasyOCR)
- Ruby – RubyGems package, idiomatic Ruby API, native bindings
- PHP – Composer package, modern PHP 8.4+ support, type-safe API, async extraction
- Elixir – Hex package, OTP integration, concurrent processing
- R – r-universe package, idiomatic R API, extendr bindings
JavaScript/TypeScript:
- @kreuzberg/node – Native NAPI-RS bindings for Node.js/Bun, fastest performance
- @kreuzberg/wasm – WebAssembly for browsers/Deno/Cloudflare Workers, full feature parity (PDF, Excel, OCR, archives)
Compiled Languages:
- Go – Go module with FFI bindings, context-aware async
- Java – Maven Central, Foreign Function & Memory API
- C# – NuGet package, .NET 6.0+, full async/await support
Native:
- Rust – Core library, flexible feature flags, zero-copy APIs
- C (FFI) – C header + shared library, pkg-config/CMake support, cross-platform
Containers:
- Docker – Official images with API, CLI, and MCP server modes (Core: ~1.0-1.3GB, Full: ~1.0-1.3GB with OCR + legacy format support)
Command-Line:
- CLI – Cross-platform binary, batch processing, MCP server mode
All language bindings include precompiled binaries for both x86_64 and aarch64 architectures on Linux and macOS.
Complete architecture coverage across all language bindings:
| Language | Linux x86_64 | Linux aarch64 | macOS ARM64 | Windows x64 |
|---|---|---|---|---|
| Python | ✅ | ✅ | ✅ | ✅ |
| Node.js | ✅ | ✅ | ✅ | ✅ |
| WASM | ✅ | ✅ | ✅ | ✅ |
| Ruby | ✅ | ✅ | ✅ | - |
| R | ✅ | ✅ | ✅ | ✅ |
| Elixir | ✅ | ✅ | ✅ | ✅ |
| Go | ✅ | ✅ | ✅ | ✅ |
| Java | ✅ | ✅ | ✅ | ✅ |
| C# | ✅ | ✅ | ✅ | ✅ |
| PHP | ✅ | ✅ | ✅ | ✅ |
| Rust | ✅ | ✅ | ✅ | ✅ |
| C (FFI) | ✅ | ✅ | ✅ | ✅ |
| CLI | ✅ | ✅ | ✅ | ✅ |
| Docker | ✅ | ✅ | ✅ | - |
Note: ✅ = Precompiled binaries available with instant installation. WASM runs in any environment with WebAssembly support (browsers, Deno, Bun, Cloudflare Workers). All platforms are tested in CI. MacOS support is Apple Silicon only.
To use embeddings functionality:
-
Install ONNX Runtime 1.24+:
- Linux: Download from ONNX Runtime releases (Debian packages may have older versions)
- MacOS:
brew install onnxruntime
- Windows: Download from ONNX Runtime releases
-
Use embeddings in your code - see Embeddings Guide
Note: Kreuzberg requires ONNX Runtime version 1.24+ for embeddings. All other Kreuzberg features work without ONNX Runtime.
91+ file formats across 8 major categories with intelligent format detection and comprehensive metadata extraction.
| Category | Formats | Capabilities |
|---|---|---|
| Word Processing | .docx , .docm , .dotx , .dotm , .dot , .odt , .pages |
Full text, tables, lists, images, metadata, styles |
| Spreadsheets | .xlsx , .xlsm , .xlsb , .xls , .xla , .xlam , .xltm , .xltx , .xlt , .ods , .numbers |
Sheet data, formulas, cell metadata, charts |
| Presentations | .pptx , .pptm , .ppsx , .potx , .potm , .pot , .key |
Slides, speaker notes, images, metadata |
.pdf |
Text, tables, images, metadata, OCR support | |
| eBooks | .epub , .fb2 |
Chapters, metadata, embedded resources |
| Database | .dbf |
Table data extraction, field type support |
| Hangul | .hwp , .hwpx |
Korean document format, text extraction |
| Category | Formats | Features |
|---|---|---|
| Raster | .png , .jpg , .jpeg , .gif , .webp , .bmp , .tiff , .tif |
OCR, table detection, EXIF metadata, dimensions, color space |
| Advanced | .jp2 , .jpx , .jpm , .mj2 , .jbig2 , .jb2 , .pnm , .pbm , .pgm , .ppm |
Pure Rust decoders (JPEG 2000, JBIG2), OCR, table detection |
| Vector | .svg |
DOM parsing, embedded text, graphics metadata |
| Category | Formats | Features |
|---|---|---|
| Markup | .html , .htm , .xhtml , .xml , .svg |
DOM parsing, metadata (Open Graph, Twitter Card), link extraction |
| Structured Data | .json , .yaml , .yml , .toml , .csv , .tsv |
Schema detection, nested structures, validation |
| Text & Markdown | .txt , .md , .markdown , .djot , .mdx , .rst , .org , .rtf |
CommonMark, GFM, Djot, MDX, reStructuredText, Org Mode, Rich Text |
| Category | Formats | Features |
|---|---|---|
.eml , .msg |
Headers, body (HTML/plain), attachments, UTF-16 support | |
| Archives | .zip , .tar , .tgz , .gz , .7z |
Recursive extraction, nested archives, metadata |
| Category | Formats | Features |
|---|---|---|
| Citations | .bib , .ris , .nbib , .enw , .csl |
BibTeX/BibLaTeX, RIS, PubMed/MEDLINE, EndNote XML, CSL JSON |
| Scientific | .tex , .latex , .typ , .typst , .jats , .ipynb |
LaTeX, Typst, JATS journal articles, Jupyter notebooks |
| Publishing | .fb2 , .docbook , .dbk , .opml |
FictionBook, DocBook XML, OPML outlines |
| Documentation | .pod , .mdoc , .troff |
Perl POD, man pages, troff |
| Feature | Description |
|---|---|
| Structure Extraction | Functions, classes, methods, structs, interfaces, enums |
| Import/Export Analysis | Module dependencies, re-exports, wildcard imports |
| Symbol Extraction | Variables, constants, type aliases, properties |
| Docstring Parsing | Google, NumPy, Sphinx, JSDoc, RustDoc, and 10+ formats |
| Diagnostics | Parse errors with line/column positions |
| Syntax-Aware Chunking | Split code by semantic boundaries, not arbitrary byte offsets |
Powered by tree-sitter-language-pack with dynamic grammar download. See TSLP documentation for the full language list.
OCR with Table Extraction
Multiple OCR backends (Tesseract, EasyOCR, PaddleOCR) with intelligent table detection and reconstruction. Extract structured data from scanned documents and images with configurable accuracy thresholds.
Batch Processing
Process multiple documents concurrently with configurable parallelism. Optimize throughput for large-scale document processing workloads with automatic resource management.
Password-Protected PDFs
Handle encrypted PDFs with single or multiple password attempts. Supports both RC4 and AES encryption with automatic fallback strategies.
Language Detection
Automatic language detection in extracted text using fast-langdetect. Configure confidence thresholds and access per-language statistics.
Metadata Extraction
Extract comprehensive metadata from all supported formats: authors, titles, creation dates, page counts, EXIF data, and format-specific properties.
Kreuzberg ships with an Agent Skill that teaches AI coding assistants how to use the library correctly. It works with Claude Code, Codex, Gemini CLI, Cursor, VS Code, Amp, Goose, Roo Code, and any tool supporting the Agent Skills standard.
Install the skill into any project using the Vercel Skills CLI:
npx skills add kreuzberg-dev/kreuzberg
The skill is located at skills/kreuzberg/SKILL.md
and is automatically discovered by supported AI coding tools once installed.
- Installation Guide – Setup and dependencies
- User Guide – Comprehensive usage guide
- API Reference – Complete API documentation
- Format Support – Supported file formats
- OCR Backends – OCR engine setup
- CLI Guide – Command-line usage
- Migration Guides – Upgrading from other libraries
Contributions are welcome! See CONTRIBUTING.md for guidelines.
Elastic License 2.0 (ELv2) - see LICENSE for details. See https://www.elastic.co/licensing/elastic-license for the full license text.

---

