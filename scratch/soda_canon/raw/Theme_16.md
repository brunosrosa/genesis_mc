# 🧠 SODA CANONICAL KNOWLEDGE BASE
> Gerado pelo @soda-knowledge-curator.
> Fontes Originais: 249 | Fontes Puras: 247 | Temas Identificados: 24



## 🧩 Eixo Temático 16

# genai-processors/pyproject.toml at main · google-gemini/genai ...
Source URL: https://github.com/google-gemini/genai-processors/blob/main/pyproject.toml

Source Type: web_page

Source ID: 251e247f-8966-4271-b4b0-0f03b600d63a


-
Notifications
You must be signed in to change notification settings - Fork 212
Expand file tree
/
Copy pathpyproject.toml
More file actions
97 lines (89 loc) · 2.66 KB
/
pyproject.toml
File metadata and controls
97 lines (89 loc) · 2.66 KB
1
2
3
4
5
6
7
8
9
10
11
12
13
14
15
16
17
18
19
20
21
22
23
24
25
26
27
28
29
30
31
32
33
34
35
36
37
38
39
40
41
42
43
44
45
46
47
48
49
50
51
52
53
54
55
56
57
58
59
60
61
62
63
64
65
66
67
68
69
70
71
72
73
74
75
76
77
78
79
80
81
82
83
84
85
86
87
88
89
90
91
92
93
94
95
96
[project]
name = "genai_processors"
description = "GenAI Processors Library"
readme = "README.pypi.md"
license = {file = "LICENSE"}
requires-python = ">=3.11"
authors = [{name = "Google DeepMind", email="noreply@google.com"}]
classifiers = [ # List of https://pypi.org/classifiers/
"Intended Audience :: Developers",
"License :: OSI Approved :: Apache Software License",
"Operating System :: OS Independent",
"Programming Language :: Python",
"Programming Language :: Python :: 3",
"Programming Language :: Python :: 3.11",
"Programming Language :: Python :: 3.12",
"Programming Language :: Python :: 3.13",
"Topic :: Scientific/Engineering :: Artificial Intelligence",
"Topic :: Software Development :: Libraries :: Python Modules",
]
keywords = []
# pip dependencies of the project
# Installed locally with `pip install -e .`
dependencies = [
"absl-py>=1.0.0",
"aiofiles>=25.1.0",
"bs4>=0.0.2",
"cachetools>=6.0.0",
"dataclasses-json>=0.6.0",
"docstring-parser>=0.17.0",
"google-genai>=1.16.0",
"google-api-python-client>=0.6.0",
"google-cloud-texttospeech>=2.27.0",
"google-cloud-speech>=2.33.0",
"httpx>=0.24.0",
"jinja2>=3.0.0",
"opencv-python>=2.0.0",
"numpy>=2.0.0",
"pdfrw>=0.4",
"Pillow>=9.0.0",
"termcolor>=3.0.0",
"pypdfium2>=4.30.0",
"shortuuid>=1.0.0",
"xxhash>=3.0.0",
"mcp>=1.26.0",
"webrtcvad>=2.0.10",
]
# `version` is automatically set by flit to use `genai_processors.__version__`
dynamic = ["version"]
[project.urls]
repository = "https://github.com/google-gemini/genai-processors"
[project.optional-dependencies]
# Dependencies of processors in contrib. To avoid dependency bloat we do not
# include them in the main dependency list. But they can be installed with
# `pip install -e .[contrib]`
contrib = [
"langchain-core>=0.3.68",
"langchain-google-genai>=2.1.7",
]
# Development deps (unittest, linting, formating,...)
# Installed through `pip install -e .[dev]`
dev = [
"aiosqlite",
"av",
"flake8",
"google-adk",
"pyink",
"pylint>=2.6.0",
"pytest",
"pytest-xdist",
"torch",
"transformers",
"typing_extensions",
]
[tool.pyink]
# Formatting configuration to follow Google style-guide
line-length = 80
preview = true
pyink-indentation = 2
pyink-use-majority-quotes = true
[build-system]
# Build system specify which backend is used to build/install the project (flit,
# poetry, setuptools,...). All backends are supported by `pip install`
requires = ["flit_core >=3.8,<4"]
build-backend = "flit_core.buildapi"
[tool.flit.module]
name = "genai_processors"
dir = "."
[tool.flit.sdist]
exclude = ["tests/", "tests/*"]

---

# jcodemunch-mcp/pyproject.toml at main · jgravelle/jcodemunch-mcp ...
Source URL: https://github.com/jgravelle/jcodemunch-mcp/blob/main/pyproject.toml

Source Type: web_page

Source ID: 53f67fcd-3802-4a2c-8001-f6f8a1b516f5


-
Notifications
You must be signed in to change notification settings - Fork 272
Expand file tree
/
Copy pathpyproject.toml
More file actions
66 lines (57 loc) · 2.09 KB
/
pyproject.toml
File metadata and controls
66 lines (57 loc) · 2.09 KB
1
2
3
4
5
6
7
8
9
10
11
12
13
14
15
16
17
18
19
20
21
22
23
24
25
26
27
28
29
30
31
32
33
34
35
36
37
38
39
40
41
42
43
44
45
46
47
48
49
50
51
52
53
54
55
56
57
58
59
60
61
62
63
64
65
66
[project]
name = "jcodemunch-mcp"
version = "1.72.0"
description = "Token-efficient MCP server for source code exploration via tree-sitter AST parsing"
readme = "README.md"
requires-python = ">=3.10"
authors = [
{ name = "J. Gravelle", email = "j@gravelle.us" },
]
dependencies = [
"mcp>=1.10.0,<2.0.0",
"httpx>=0.27.0",
"tree-sitter-language-pack>=0.7.0,<1.0.0",
"pathspec>=0.12.0",
"pyyaml>=6.0",
]
[project.urls]
Homepage = "https://github.com/jgravelle/jcodemunch-mcp"
Repository = "https://github.com/jgravelle/jcodemunch-mcp"
"Bug Tracker" = "https://github.com/jgravelle/jcodemunch-mcp/issues"
[project.optional-dependencies]
anthropic = ["anthropic>=0.40.0"]
gemini = ["google-generativeai>=0.8.0"]
openai = ["openai>=1.0.0"]
minimax = ["openai>=1.0.0"]
zhipu = ["openai>=1.0.0"]
dbt = ["pyyaml>=6.0"]
http = ["uvicorn>=0.20.0", "starlette>=0.27.0", "anyio>=4.0.0"]
watch = ["watchfiles>=1.0.0"]
semantic = ["sentence-transformers>=2.2.0"]
local-embed = ["onnxruntime>=1.16.0"]
groq = ["openai>=1.0.0"]
groq-voice = ["openai>=1.0.0", "sounddevice>=0.4.6", "numpy>=1.24.0"]
groq-explain = ["openai>=1.0.0", "Pillow>=10.0.0"]
bench = ["openai>=1.0.0", "anthropic>=0.40.0", "pyyaml>=6.0", "rich>=13.0", "jinja2>=3.1"]
all = ["anthropic>=0.40.0", "google-generativeai>=0.8.0", "openai>=1.0.0", "pyyaml>=6.0", "uvicorn>=0.20.0", "starlette>=0.27.0", "anyio>=4.0.0", "watchfiles>=1.0.0", "sentence-transformers>=2.2.0", "onnxruntime>=1.16.0", "sounddevice>=0.4.6", "numpy>=1.24.0", "Pillow>=10.0.0", "rich>=13.0", "jinja2>=3.1"]
[project.scripts]
jcodemunch-mcp = "jcodemunch_mcp.server:main"
gcm = "jcodemunch_mcp.groq.cli:main"
munch-bench = "munch_bench.cli:main"
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
[tool.hatch.build.targets.wheel]
packages = ["src/jcodemunch_mcp", "munch-bench/munch_bench"]
[tool.hatch.build.targets.sdist]
exclude = [".claude/", "index.php"]
[tool.pytest.ini_options]
testpaths = ["tests"]
asyncio_mode = "auto"
[dependency-groups]
dev = [
"pytest>=9.0.2",
"pytest-asyncio>=1.3.0",
"pytest-cov>=7.0.0",
"hypothesis>=6.0.0",
]

---

# open-terminal/pyproject.toml at main · open-webui/open-terminal ...
Source URL: https://github.com/open-webui/open-terminal/blob/main/pyproject.toml

Source Type: web_page

Source ID: 945d3e65-07a5-4043-b986-8adad322a4c2


-
Notifications
You must be signed in to change notification settings - Fork 178
Expand file tree
/
Copy pathpyproject.toml
More file actions
45 lines (40 loc) · 953 Bytes
/
pyproject.toml
File metadata and controls
45 lines (40 loc) · 953 Bytes
1
2
3
4
5
6
7
8
9
10
11
12
13
14
15
16
17
18
19
20
21
22
23
24
25
26
27
28
29
30
31
32
33
34
35
36
37
38
39
40
41
42
43
44
45
[project]
name = "open-terminal"
version = "0.11.34"
description = "A remote terminal API."
license = "MIT"
readme = "README.md"
authors = [
{ name = "Timothy Jaeryang Baek", email = "tim@openwebui.com" }
]
requires-python = ">=3.11"
dependencies = [
"fastapi>=0.115.0",
"uvicorn[standard]>=0.34.0",
"click>=8.1.0",
"httpx>=0.27.0",
"python-multipart>=0.0.22",
"aiofiles>=25.1.0",
"pypdf>=5.0.0",
"python-docx>=1.0.0",
"openpyxl>=3.1.0",
"python-pptx>=1.0.0",
"striprtf>=0.0.26",
"xlrd>=2.0.0",
"nbclient>=0.10.0",
"ipykernel>=6.0.0",
"pywinpty>=2.0.0; sys_platform == 'win32'",
]
[project.optional-dependencies]
mcp = ["fastmcp>=2.0.0"]
[project.scripts]
open-terminal = "open_terminal.cli:main"
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
[tool.hatch.build.targets.wheel]
packages = ["open_terminal"]
[dependency-groups]
dev = [
"pytest>=9.0.2",
]

---

# Cargo.toml - BloopAI/vibe-kanban - GitHub
Source URL: https://github.com/BloopAI/vibe-kanban/blob/main/Cargo.toml

Source Type: web_page

Source ID: 995d9948-aba3-4bc6-a718-36eec4727d64


-
Notifications
You must be signed in to change notification settings - Fork 2.6k
Expand file tree
/
Copy pathCargo.toml
More file actions
60 lines (58 loc) · 2.01 KB
/
Cargo.toml
File metadata and controls
60 lines (58 loc) · 2.01 KB
1
2
3
4
5
6
7
8
9
10
11
12
13
14
15
16
17
18
19
20
21
22
23
24
25
26
27
28
29
30
31
32
33
34
35
36
37
38
39
40
41
42
43
44
45
46
47
48
49
50
51
52
53
54
55
56
57
58
59
60
[workspace]
resolver = "3"
members = [
"crates/api-types",
"crates/relay-client",
"crates/server",
"crates/relay-types",
"crates/trusted-key-auth",
"crates/mcp",
"crates/db",
"crates/executors",
"crates/services",
"crates/worktree-manager",
"crates/workspace-manager",
"crates/relay-control",
"crates/relay-protocol",
"crates/relay-ws",
"crates/ws-bridge",
"crates/relay-hosts",
"crates/client-info",
"crates/remote-info",
"crates/utils",
"crates/git",
"crates/git-host",
"crates/local-deployment",
"crates/deployment",
"crates/review",
"crates/embedded-ssh",
"crates/desktop-bridge",
"crates/tauri-app",
"crates/preview-proxy",
"crates/relay-tunnel-core",
"crates/relay-webrtc",
]
exclude = ["crates/remote", "crates/relay-tunnel"]
[workspace.dependencies]
tokio = { version = "1.0", features = ["full"] }
axum = { version = "0.8.4", features = ["macros", "multipart", "ws"] }
tower-http = { version = "0.5", features = ["cors", "request-id", "trace", "fs", "validate-request", "compression-gzip", "compression-br"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = { version = "1.0", features = ["preserve_order"] }
anyhow = "1.0"
git2 = { version = "0.20.3", default-features = false }
reqwest = { version = "0.13", default-features = false, features = ["json", "query", "stream", "rustls"] }
rustls = { version = "0.23", default-features = false, features = ["aws_lc_rs", "std", "tls12"] }
thiserror = "2.0.12"
tracing = "0.1.43"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "json"] }
ts-rs = { git = "https://github.com/xazukx/ts-rs.git", branch = "use-ts-enum", features = ["uuid-impl", "chrono-impl", "no-serde-warnings", "serde-json-impl"] }
schemars = { version = "1.0.4", features = ["derive", "chrono04", "uuid1", "preserve_order"] }
serde_with = "3"
async-trait = "0.1"
aws-lc-sys = "=0.37.0"
aws-lc-rs = "=1.16.0"
[profile.release]
debug = 1
split-debuginfo = "packed"
strip = true

---

