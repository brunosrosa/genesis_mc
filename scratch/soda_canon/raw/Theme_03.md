# 🧠 SODA CANONICAL KNOWLEDGE BASE
> Gerado pelo @soda-knowledge-curator.
> Fontes Originais: 249 | Fontes Puras: 247 | Temas Identificados: 24



## 🧩 Eixo Temático 3

# A Model Context Protocol (MCP) server that provides web search capabilities through DuckDuckGo, with additional features for content fetching and parsing. - GitHub
Source URL: https://github.com/nickclyde/duckduckgo-mcp-server

Source Type: web_page

Source ID: 254d3f0c-58d4-4020-925f-09ef3f0b1a30


A Model Context Protocol (MCP) server that provides web search capabilities through DuckDuckGo, with additional features for content fetching and parsing.
uvx duckduckgo-mcp-server
- Web Search: Search DuckDuckGo with advanced rate limiting and result formatting
- Content Fetching: Retrieve and parse webpage content with intelligent text extraction
- Rate Limiting: Built-in protection against rate limits for both search and content fetching
- Error Handling: Comprehensive error handling and logging
- LLM-Friendly Output: Results formatted specifically for large language model consumption
Install from PyPI using uv
:
uv pip install duckduckgo-mcp-server
- Download Claude Desktop
- Create or edit your Claude Desktop configuration:
- On macOS:
~/Library/Application Support/Claude/claude_desktop_config.json
- On Windows:
%APPDATA%\Claude\claude_desktop_config.json
- On macOS:
Add the following configuration:
Basic Configuration (No SafeSearch, No Default Region):
{
"mcpServers": {
"ddg-search": {
"command": "uvx",
"args": ["duckduckgo-mcp-server"]
}
}
}
With SafeSearch and Region Configuration:
{
"mcpServers": {
"ddg-search": {
"command": "uvx",
"args": ["duckduckgo-mcp-server"],
"env": {
"DDG_SAFE_SEARCH": "STRICT",
"DDG_REGION": "cn-zh"
}
}
}
}
Configuration Options:
DDG_SAFE_SEARCH
: SafeSearch filtering level (optional)STRICT
: Maximum content filtering (kp=1)MODERATE
: Balanced filtering (kp=-1, default if not specified)OFF
: No content filtering (kp=-2)
DDG_REGION
: Default region/language code (optional, examples below)us-en
: United States (English)cn-zh
: China (Chinese)jp-ja
: Japan (Japanese)wt-wt
: No specific region- Leave empty for DuckDuckGo's default behavior
- Restart Claude Desktop
- Download Claude Code
- Ensure
uvenv
is installed and theuvx
command is available - Add the MCP server:
claude mcp add ddg-search uvx duckduckgo-mcp-server
The server supports alternative transports for use with other MCP clients:
# SSE transport
uvx duckduckgo-mcp-server --transport sse
# Streamable HTTP transport
uvx duckduckgo-mcp-server --transport streamable-http
The default transport is stdio
, which is used by Claude Desktop and Claude Code.
For local development:
# Install dependencies
uv sync
# Run with the MCP Inspector
mcp dev src/duckduckgo_mcp_server/server.py
# Install locally for testing with Claude Desktop
mcp install src/duckduckgo_mcp_server/server.py
# Run all tests
uv run python -m pytest src/duckduckgo_mcp_server/ -v
# Run only unit tests
uv run python -m pytest src/duckduckgo_mcp_server/test_server.py -v
# Run only e2e tests
uv run python -m pytest src/duckduckgo_mcp_server/test_e2e.py -v
async def search(query: str, max_results: int = 10, region: str = "") -> str
Performs a web search on DuckDuckGo and returns formatted results.
Parameters:
query
: Search query stringmax_results
: Maximum number of results to return (default: 10)region
: (Optional) Region/language code to override the default. Leave empty to use the configured default region.
Region Code Examples:
us-en
: United States (English)cn-zh
: China (Chinese)jp-ja
: Japan (Japanese)de-de
: Germany (German)fr-fr
: France (French)wt-wt
: No specific region
Returns: Formatted string containing search results with titles, URLs, and snippets.
Example Usage:
- Search with default settings:
search("python tutorial")
- Search with specific region:
search("latest news", region="jp-ja")
for Japanese news
async def fetch_content(url: str) -> str
Fetches and parses content from a webpage.
Parameters:
url
: The webpage URL to fetch content from
Returns: Cleaned and formatted text content from the webpage.
- Search: Limited to 30 requests per minute
- Content Fetching: Limited to 20 requests per minute
- Automatic queue management and wait times
- Removes ads and irrelevant content
- Cleans up DuckDuckGo redirect URLs
- Formats results for optimal LLM consumption
- Truncates long content appropriately
-
SafeSearch Filtering: Configured at server startup via
DDG_SAFE_SEARCH
environment variable- Controlled by administrators, not modifiable by AI assistants
- Filters inappropriate content based on the selected level
- Uses DuckDuckGo's official
kp
parameter
-
Region Localization:
- Default region set via
DDG_REGION
environment variable - Can be overridden per search request by AI assistants
- Improves result relevance for specific geographic regions
- Default region set via
- Comprehensive error catching and reporting
- Detailed logging through MCP context
- Graceful degradation on rate limits or timeouts
Issues and pull requests are welcome! Some areas for potential improvement:
- Enhanced content parsing options
- Caching layer for frequently accessed content
- Additional rate limiting strategies
This project is licensed under the MIT License.

---

# GitHub - kouui/web-search-duckduckgo: MCP server of web search/fetch functionality using duckduckgo and jina api. no api key required.
Source URL: https://github.com/kouui/web-search-duckduckgo

Source Type: web_page

Source ID: 99b99cdc-1ad6-4b85-af78-117d52da7db3


This project provides an MCP (Model Context Protocol) server that allows you to search the web using the DuckDuckGo search engine and optionally fetch and summarize the content of the found URLs.
- Web Search: Search the web using DuckDuckGo.
- Result Extraction: Extracts titles, URLs, and snippets from search results.
- Content Fetching (Optional): Fetches the content of the URLs found in the search results and converts it to markdown format using jina api.
- Parallel Fetching: Fetches multiple URLs concurrently for faster processing.
- Error Handling: Gracefully handles timeouts and other potential errors during search and fetching.
- Configurable: Allows you to set the maximum number of search results to return.
- Jina API: using jina api to convert html to markdown.
- MCP Compliant: This server is designed to be used with any MCP-compatible client.
-
Prerequisites:
uvx
package manager
-
Claude Desktop Configuration
- If you are using Claude Desktop, you can add the server to the
claude_desktop_config.json
file.
{ "mcpServers": { "web-search-duckduckgo": { "command": "uvx", "args": [ "--from", "git+https://github.com/kouui/web-search-duckduckgo.git@main", "main.py" ] } } }
the above configuration is not working, you might need to clone the repository to local pc and use the following configuration
{ "mcpServers": { "web-search-duckduckgo": { "command": "uv", "args": [ "--directory", "/path/to/web-search-duckduckgo", "run", "main.py" ] } } }
- If you are using Claude Desktop, you can add the server to the
-
Tool
-
In your MCP client (e.g., Claude), you can now use the following tools:
-
search_and_fetch
: Search the web and fetch the content of the URLs.query
: The search query string.limit
: The maximum number of results to return (default: 3, maximum: 10).
-
fetch
: Fetch the content of a specific URL.url
: The URL to fetch.
-
This project is licensed under the MIT License. (Add a license file if you want to specify a license).

---

# _Output_AgentGateway_e_MCPs
Source URL: None
Source Type: generated_text
Source ID: c127fe9d-a0fa-4319-ab0a-8ad97ca51081

{
  "value": {
    "content": "_Output_AgentGateway_e_MCPs\n\nagent-gateway\n\nEnabled\n\n1. jcodemunch_index_repo\n\nIndex a GitHub repository's source code. Fetches files, parses ASTs, extracts symbols, and saves to local storage. Set JCODEMUNCH_USE_AI_SUMMARIES=false to disable AI summaries globally.\n\n2. jcodemunch_index_folder\n\nIndex a local folder containing source code. Response includes `discovery_skip_counts` (files filtered per reason), `no_symbols_count`/`no_symbols_files` (files with no extractable symbols) for diagnosing missing files. Set JCODEMUNCH_USE_AI_SUMMARIES=false to disable AI summaries globally.\n\n3. jcodemunch_get_file_tree\n\nGet the file tree of an indexed repository, optionally filtered by path prefix. Results are capped at max_files (default 500) to prevent token overflow; use path_prefix to scope large trees.\n\n4. jcodemunch_get_file_outline\n\nGet all symbols (functions, classes, methods) in a file with full signatures (including parameter names) and summaries. Use signatures to review naming at parameter granularity without reading the full file. Pass repo and file_path (e.g. 'src/main.py').\n\n5. jcodemunch_get_symbol_source\n\nGet full source of one symbol (symbol_id → flat object) or many (symbol_ids[] → {symbols, errors}). Supports verify, context_lines, and fqn (PHP FQN via PSR-4).\n\n6. jcodemunch_get_file_content\n\nGet cached source for a file, optionally sliced to a line range.\n\n7. jcodemunch_search_symbols\n\nSearch for symbols matching a query across the entire indexed repository. Returns matches with signatures and summaries.\n\n8. time_server_get_current_time\n\nGet current time in a specific timezone\n\n9. duckduckgo_search_fetch_content\n\nFetch and extract the main text content from a webpage. Strips out navigation, headers, footers, scripts, and styles to return clean readable text. Use this after searching to read the full content of a specific result. Supports pagination for long pages via start_index and max_length. Args: url: The full URL of the webpage to fetch (must start with http:// or https://). start_index: Character offset to start reading from (default: 0). Use this to paginate through long content. max_length: Maximum number of characters to return (default: 8000). Increase for more content per request or decrease for quicker responses. ctx: MCP context for logging.\n\n10. github_mcp_search_repositories\n\nSearch for GitHub repositories\n\n11. github_mcp_get_file_contents\n\nGet the contents of a file or directory from a GitHub repository\n\n12. github_mcp_search_issues\n\nSearch for issues and pull requests across GitHub repositories\n\n13. github_mcp_get_issue\n\nGet details of a specific issue in a GitHub repository.\n\n14. github_mcp_get_pull_request\n\nGet details of a specific pull request\n\n15. github_mcp_list_pull_requests\n\nList and filter repository pull requests\n\n16. notebooklm_notebook_query\n\nAsk AI about EXISTING sources already in notebook. NOT for finding new sources. Use research_start instead for: deep research, web search, find new sources, Drive search.\n\n17. notebooklm_notebook_list\n\nList all notebooks.\n\n18. notebooklm_notebook_get\n\nGet notebook details with sources.\n\n19. notebooklm_source_get_content\n\nGet raw text content of a source (no AI processing). Returns the original indexed text from PDFs, web pages, pasted text, or YouTube transcripts. Much faster than notebook_query for content export.\n\n20. sqlite_soda_read_query\n\nExecute a SELECT query on the SQLite database\n\n21. sqlite_soda_list_tables\n\nList all tables in the SQLite database\n\n22. sqlite_soda_describe_table\n\nGet the schema information for a specific table\n\n23. docs_scraper_get_docs_tree\n\nGet file/folder structure of cached docs. Use to discover file paths before get_docs_content. Optionally filter by path or limit depth.\n\n24. docs_scraper_search_docs\n\nFull-text search within cached docs. FASTEST way to find information—use before get_docs_content. Returns ranked results with file paths and snippets.",
    "title": "_Output_AgentGateway_e_MCPs",
    "source_type": "generated_text",
    "url": null,
    "char_count": 3928
  }
}


---

# jgravelle/jcodemunch-mcp: Token-efficient MCP server for ... - GitHub
Source URL: https://github.com/jgravelle/jcodemunch-mcp

Source Type: web_page

Source ID: c560b851-bac9-48f4-8f6d-24114f78c3f6


Quickstart - https://github.com/jgravelle/jcodemunch-mcp/blob/main/QUICKSTART.md
A crapload of detailed info: http://jcodemunch.com/
Use it to make money, and Uncle J. gets a taste. Fair enough? details
| Doc | What it covers |
|---|---|
| QUICKSTART.md | Zero-to-indexed in three steps |
| USER_GUIDE.md | Full tool reference, workflows, and best practices |
| AGENT_HOOKS.md | Agent hooks and prompt policies |
| CONFIGURATION.md | JSONC config file reference, migration from env vars |
| GROQ.md | Groq Remote MCP integration, deployment, gcm CLI |
| ARCHITECTURE.md | Internal design, storage model, and extension points |
| LANGUAGE_SUPPORT.md | Supported languages and parsing details |
| CONTEXT_PROVIDERS.md | dbt, Git, and custom context provider docs |
| TROUBLESHOOTING.md | Common issues and fixes |
Most AI agents explore repositories the expensive way:
open entire files → skim thousands of irrelevant lines → repeat.
That is not “a little inefficient.” That is a token incinerator.
jCodeMunch indexes a codebase once and lets agents retrieve only the exact code they need: functions, classes, methods, constants, outlines, and tightly scoped context bundles, with byte-level precision.
In retrieval-heavy workflows, that routinely cuts code-reading token usage by 95%+ because the agent stops brute-reading giant files just to find one useful implementation.
| Task | Traditional approach | With jCodeMunch |
|---|---|---|
| Find a function | Open and scan large files | Search symbol → fetch exact implementation |
| Understand a module | Read broad file regions | Pull only relevant symbols and imports |
| Explore repo structure | Traverse file after file | Query outlines, trees, and targeted bundles |
Index once. Query cheaply. Keep moving. Precision context beats brute-force context.
Retrieval decides what to send. MUNCH decides how to pack it.
Every tool response can be emitted in a purpose-built compact wire format instead of verbose JSON. Path prefixes are interned to short handles, homogeneous lists of dicts pack into single-character-tagged CSV rows, and per-column types are preserved so the decode is lossless.
# any tool call accepts format=
find_references(identifier="get_user", format="auto")
# auto — emit compact if savings ≥ 15%, otherwise JSON
# compact — always compact
# json — never compact (back-compat passthrough)
Benchmark (v1.56.0): median 45.5% bytes saved across 6 representative tools, peaks at 55.4% on graph and outline responses. Full spec in SPEC_MUNCH.md; numbers and harness in TOKEN_SAVINGS.md.
Encoding savings stack on top of retrieval savings — every byte off the wire is a byte the agent doesn't pay to read.
- Artur Skowroński (VirtusLab) — "roughly 80% fewer tokens, or 5× more efficient — index once, query cheaply forever" · GitHub All-Stars #15
- Julian Horsey (Geeky Gadgets) — "3,850 tokens reduced to just 700 — a 5.5× improvement" · JCodeMunch AI Token Saver
- Sion Williams — "preserving tokens for tasks that actually require reasoning rather than retrieval" · March 2026 AI Workflow Update
- Traci Lim (AWS · ASEAN AI Lead) — "structural queries that native tools can't answer: find_importers, get_blast_radius, get_class_hierarchy, find_dead_code" · 5 Repos That Save Token Usage in Claude Code
- Eric Grill — "context is the scarce resource. Cut it by 90% and the whole stack gets cheaper and more reliable" · jCodemunch: Context Engine for AI Agents
jCodeMunch-MCP is free for non-commercial use.
Commercial use requires a paid license.
jCodeMunch-only licenses
- Builder — $79 — 1 developer
- Studio — $349 — up to 5 developers
- Platform — $1,999 — org-wide internal deployment
Want both code and docs retrieval?
Stop paying your model to read the whole damn file.
jCodeMunch turns repo exploration into structured retrieval.
Instead of forcing an agent to open giant files, wade through imports, boilerplate, comments, helpers, and unrelated code, jCodeMunch lets it navigate by what the code is and retrieve only what matters.
That means:
- 95%+ lower code-reading token usage in many retrieval-heavy workflows
- less irrelevant context polluting the prompt
- faster repo exploration
- more accurate code lookup
- less repeated file-scanning nonsense
It indexes your codebase once using tree-sitter, stores structured symbol metadata plus byte offsets into the original source, and retrieves exact implementations on demand instead of re-reading entire files over and over.
Recent releases have made that retrieval workflow sharper and more useful in real engineering work, with BM25-based symbol search, fuzzy matching, semantic/hybrid search (opt-in, zero mandatory dependencies), query-driven token-budgeted context assembly (get_ranked_context
), dead code detection (find_dead_code
), untested symbol detection (get_untested_symbols
), git-diff-to-symbol mapping (get_changed_symbols
), architectural centrality ranking (get_symbol_importance
, PageRank), blast-radius depth scoring with source snippets, context bundles with token budgets, AST-derived call graphs and call hierarchy traversal, decorator-aware search and filtering, hotspot detection (complexity x churn), dependency cycles and coupling metrics, session-aware routing (plan_turn
, turn budgets, negative evidence), agent config auditing, complexity-based model routing (Agent Selector), enforcement hooks (PreToolUse/PostToolUse/PreCompact), dependency graphs, class hierarchy traversal, multi-symbol bundles, live watch-based reindexing, automatic Claude Code worktree discovery (watch-claude
), auto-watch on demand (when watch: true
in config, the server automatically indexes and watches any repo a tool is called against — ensuring fresh results from the first call), trusted-folder access controls, edit-ready refactoring plans (plan_refactoring
) for rename, move, extract, and signature change operations, symbol provenance archaeology (get_symbol_provenance
— full git lineage, semantic commit classification, evolution narrative), unified PR risk profiling (get_pr_risk_profile
— composite risk score fusing blast radius, complexity, churn, test gaps, and volume), automatic response secret redaction (AWS/GCP/Azure/JWT/GitHub tokens scrubbed before reaching the LLM context window), and cross-language AST pattern matching (search_ast
— 10 preset anti-pattern detectors + custom mini-DSL for structural queries like call:*.unwrap
, string:/password/i
, nesting:5+
; works across all 70+ languages with universal node-type mapping).
Measured with tiktoken cl100k_base
across three public repos. Workflow: search_symbols
(top 5) + get_symbol_source
× 3 per query. Baseline: all source files concatenated (minimum cost for an agent that reads everything). Full methodology and harness →
| Repository | Files | Symbols | Baseline tokens | jCodeMunch tokens | Reduction |
|---|---|---|---|---|---|
| expressjs/express | 34 | 117 | 73,838 | ~1,300 avg | 98.4% |
| fastapi/fastapi | 156 | 1,359 | 214,312 | ~15,600 avg | 92.7% |
| gin-gonic/gin | 40 | 805 | 84,892 | ~1,730 avg | 98.0% |
| Grand total (15 task-runs) | 1,865,210 | 92,515 | 95.0% |
Per-query results range from 79.7% (dense FastAPI router query) to 99.8% (sparse context-bind query on Express). The 95% figure is the aggregate. Run python benchmarks/harness/run_benchmark.py
to reproduce.
Independent 50-iteration A/B test on a real Vue 3 + Firebase production codebase — JCodeMunch vs native tools (Grep/Glob/Read), Claude Sonnet 4.6, fresh session per iteration:
| Metric | Native | JCodeMunch |
|---|---|---|
| Success rate | 72% | 80% |
| Timeout rate | 40% | 32% |
| Mean cost/iteration | $0.783 | $0.738 |
| Mean cache creation | 104,135 | 93,178 (−10.5%) |
Tool-layer savings isolated from fixed overhead: 15–25%. One finding category appeared exclusively in the JCodeMunch variant: orphaned file detection via find_importers
— a structural query native tools cannot answer without scripting.
Full report: benchmarks/ab-test-naming-audit-2026-03-18.md
Most agents still inspect codebases like tourists trapped in an airport gift shop:
- open entire files to find one function
- re-read the same code repeatedly
- consume imports, boilerplate, and unrelated helpers
- burn context window on material they never needed in the first place
jCodeMunch fixes that by giving them a structured way to:
- search symbols by name, kind, or language — with fuzzy matching and optional semantic/hybrid search
- inspect file and repo outlines before pulling source
- retrieve exact symbol implementations only
- grab a token-budgeted context bundle or ranked context pack for a task
- fall back to text search when structure alone is not enough
- detect dead code, trace impact, rank by centrality, and map git diffs to symbols
- plan the next turn with
plan_turn
— confidence-guided routing before the first read - track session state and avoid re-reading files the agent already explored
Agents do not need bigger and bigger context windows.
They need better aim.
Find and fetch functions, classes, methods, constants, and more without opening entire files.
Inspect repository structure and file outlines before asking for source.
Send the model the code it needs, not 1,500 lines of collateral damage.
find_importers
tells you what imports a file. get_blast_radius
tells you what breaks if you change a symbol, with depth-weighted risk scores and optional source snippets. get_class_hierarchy
traverses inheritance chains. get_call_hierarchy
traces callers and callees N levels deep using AST-derived call graphs, with optional LSP-enriched dispatch resolution for interface/trait method calls. find_dead_code
finds symbols and files unreachable from any entry point. get_untested_symbols
finds functions with no evidence of test-file reachability — the intersection of import-graph analysis and test-file detection. get_changed_symbols
maps a git diff to the exact symbols that were added, modified, or removed. get_symbol_importance
ranks your codebase by architectural centrality using PageRank on the import graph. get_hotspots
surfaces the riskiest code by combining complexity with git churn. get_dependency_cycles
detects circular imports. get_coupling_metrics
measures module coupling and instability. get_tectonic_map
discovers the logical module topology by fusing three coupling signals (imports, shared references, git co-churn) — revealing hidden module boundaries, misplaced files, and god-module risk without any configuration. get_signal_chains
traces how external signals (HTTP requests, CLI commands, scheduled tasks, events) propagate through the codebase via the call graph — discovery mode maps all entry-point-to-leaf pathways and reports orphan symbols, lookup mode tells you which user-facing chains a specific symbol participates in (e.g. "validate_email sits on POST /api/users and cli:import-users"). These are not "faster grep" — they are questions grep cannot answer at all.
audit_agent_config
scans your CLAUDE.md, .cursorrules, copilot-instructions.md, and other agent config files for token waste: per-file token cost, stale symbol references (cross-referenced against the index — catches renamed or deleted functions), dead file paths, redundancy between global and project configs, bloat, and scope leaks. No other tool can tell you "line 15 references a function that was renamed three weeks ago."
get_symbol_provenance
is git archaeology: given a symbol, it traces every commit that touched it, classifies each into semantic categories (creation, bugfix, refactor, feature, perf, rename, revert), extracts commit intent, and generates a human-readable narrative explaining who created it, why, and how it evolved. get_pr_risk_profile
produces a unified risk assessment for a branch or PR — one call fuses blast radius, complexity, churn, test gaps, and change volume into a composite risk score (0.0–1.0) with actionable recommendations. All responses are automatically scanned for leaked credentials (AWS keys, JWTs, GCP service accounts, etc.) and redacted before reaching the LLM.
search_ast
brings structural code analysis to every language jCodeMunch indexes — write one query, match across all 70+ languages. Preset anti-patterns detect common problems without any configuration: empty_catch
(silently swallowed errors), bare_except
(catch-all handlers), deeply_nested
(5+ control-flow levels), nested_loops
(O(n³)+ performance risk), god_function
(100+ line functions), eval_exec
(injection-risk dynamic execution), hardcoded_secret
(credential patterns in strings), todo_fixme
(unfinished work markers), magic_number
(unexplained numeric constants), and reassigned_param
(overwritten function parameters). Run category='all'
for a full sweep, or focus on security
, error_handling
, complexity
, performance
, or maintenance
. Custom queries use a mini-DSL: call:*.unwrap
(find method calls by glob), string:/password/i
(regex over string literals), comment:/TODO/i
(regex in comments), nesting:5+
, loops:3+
, lines:80+
(threshold queries). Every match is attributed to its enclosing indexed symbol with complexity metadata — so you can see not just where the problem is, but how bad the surrounding function already is.
winnow_symbols
composes signals that every other tool exposes separately — kind, complexity, decorator, direct call references, file glob, name regex, git churn, and PageRank importance — into a single AND-intersected query. Agents stop making four or five calls and merging results by hand: "functions that call db.Exec
, cyclomatic > 10, churned in the last 30 days, ranked by importance" resolves in one round trip. Supported axes expose their own operator set (eq
, in
, matches
, contains
, numeric comparisons); the window for churn-based filters is per-criterion. Results include per-symbol importance, complexity, and churn scores so the agent can explain why each survivor made the cut.
Useful for onboarding, debugging, refactoring, impact analysis, and exploring unfamiliar repos without brute-force file reading.
plan_refactoring
generates exact edit-ready instructions for rename, move, extract, and
signature change operations. Returns {old_text, new_text}
blocks compatible with any editor's
find-and-replace, plus import rewrites, collision detection, new file generation, and multi-file coordination.
Indexes are stored locally for fast repeated access.
jCodeMunch indexes local folders or GitHub repos, parses source with tree-sitter, extracts symbols, and stores structured metadata alongside raw file content in a local index. Each symbol includes enough information to be found cheaply and retrieved precisely later.
That includes metadata like:
- signature
- kind
- qualified name
- one-line summary
- byte offsets into the original file
So when the agent wants a symbol, jCodeMunch can fetch the exact source directly instead of loading and rescanning the full file.
Ubuntu 24.04+ / Debian 12+: System Python is externally managed (PEP 668). Use
pipx install jcodemunch-mcp
oruv tool install jcodemunch-mcp
instead of barepip install
.
pip install jcodemunch-mcp
jcodemunch-mcp init
init
auto-detects your MCP clients (Claude Code, Claude Desktop, Cursor, Windsurf, Continue), writes their config entries, installs the CLAUDE.md prompt policy so your agent actually uses jCodeMunch, optionally installs enforcement hooks (PreToolUse read guard + PostToolUse auto-reindex + PreCompact session snapshot), optionally indexes your project, and audits your agent config files for token waste. Run jcodemunch-mcp init --help
for all flags.
Prefer a one-line CLAUDE.md? From v1.71.0 the server exposes a
jcodemunch_guide
tool that returns the same policy snippetclaude-md --generate
prints — with the running version embedded. Keep this single line in your CLAUDE.md / AGENT.md and the guide always matches the installed server:Call the jcodemunch_guide tool and strictly follow its instructions.The tool is force-included, so it can't be hidden by
disabled_tools
or tier filtering.
For non-interactive CI or scripting:
jcodemunch-mcp init --yes --claude-md global --hooks --index --audit
pip install jcodemunch-mcp
Want semantic search? Install the local embedding extra for zero-config semantic search — no API keys, no internet after first download:
pip install "jcodemunch-mcp[local-embed]" # bundled ONNX encoder (recommended) jcodemunch-mcp download-model # fetch model (~23 MB, one-time)Want AI-generated summaries? Install the extra for your provider:
pip install "jcodemunch-mcp[anthropic]" # Claude pip install "jcodemunch-mcp[gemini]" # Gemini pip install "jcodemunch-mcp[openai]" # OpenAI-compatible pip install "jcodemunch-mcp[all]" # all providers + local embeddingsWithout an extra, summaries fall back to signatures (which still works — you just get shorter descriptions). Run
jcodemunch-mcp config --check
to verify your provider is installed and working.
If you’re using Claude Code, pick whichever matches what you installed in step 1.
Pip install (simplest, what most people do):
claude mcp add -s user jcodemunch jcodemunch-mcp
The -s user
flag registers it at user scope so it's available in every
project. Without it, the registration is project-local and you'll see it
missing the next time you cd
elsewhere. If jcodemunch-mcp
isn't found
on PATH (common on Windows where pip install --user
installs to
AppData\Roaming\Python\PythonXYZ\Scripts\
), use the absolute path:
# Windows
claude mcp add -s user jcodemunch "C:\Users\YOU\AppData\Roaming\Python\Python312\Scripts\jcodemunch-mcp.exe"
# macOS/Linux — check `which jcodemunch-mcp` first
claude mcp add -s user jcodemunch "$(which jcodemunch-mcp)"
uvx (no pip install required, but uv must be on PATH):
claude mcp add -s user jcodemunch uvx jcodemunch-mcp
If /mcp
reports failed
with no reason, run claude --mcp-debug
or
check %USERPROFILE%\AppData\Roaming\Claude\logs\mcp*.log
— the /mcp
summary hides the actual error.
If you’re using Paperclip (the multi-agent orchestration platform), add a .mcp.json
to your workspace root:
{
"mcpServers": {
"jcodemunch": {
"type": "stdio",
"command": "uvx",
"args": ["jcodemunch-mcp"]
},
"jdocmunch": {
"type": "stdio",
"command": "uvx",
"args": ["jdocmunch-mcp"]
}
}
}
Paperclip’s Claude Code agents auto-detect .mcp.json
at startup. Add both servers to give your agents symbol search + doc navigation without blowing the token budget.
This matters more than people think.
Installing jCodeMunch makes the tools available. It does not guarantee the agent will stop its bad habit of brute-reading files unless you instruct it to prefer symbol search, outlines, and targeted retrieval. The changelog specifically calls out improved onboarding around this because it is a real source of confusion for first-time users.
A simple instruction like this helps:
Use jcodemunch-mcp for code lookup whenever available. Prefer symbol search, outlines, and targeted retrieval over reading full files.
Note:
jcodemunch-mcp init
handles steps 2 and 3 automatically. For a comprehensive guide on enforcing these rules through agent hooks and prompt policies, see AGENT_HOOKS.md.
Pre-built indexes for popular frameworks and libraries. Skip the initial indexing step — install a pack and start querying immediately.
# List available packs
jcodemunch-mcp install-pack --list
# Install a free pack
jcodemunch-mcp install-pack fastapi
# Install a licensed pack
jcodemunch-mcp install-pack express --license YOUR-KEY
Free packs require no license. Licensed packs require a jCodeMunch license. Use --force
to re-download an already-installed pack.
Use jCodeMunch as a remote MCP tool with Groq's ultra-fast inference — answer codebase questions in seconds with zero local setup.
from openai import OpenAI
client = OpenAI(api_key="YOUR_GROQ_KEY", base_url="https://api.groq.com/openai/v1")
response = client.responses.create(
model="llama-3.3-70b-versatile",
input="What does parse_file do in jgravelle/jcodemunch-mcp?",
tools=[{
"type": "mcp",
"server_label": "jcodemunch",
"server_url": "https://YOUR_JCODEMUNCH_URL",
"headers": {"Authorization": "Bearer YOUR_TOKEN"},
"server_description": "Code intelligence via tree-sitter AST parsing.",
"require_approval": "never",
}],
)
Groq handles MCP tool discovery and execution server-side — one API call, no orchestration needed.
Self-host with Docker + Caddy for auto-TLS:
DOMAIN=mcp.example.com JCODEMUNCH_HTTP_TOKEN=secret docker compose up -d
See GROQ.md for the full tutorial: allowed-tools presets, model recommendations, deployment options, and validation scripts.
Get a structured PR review in under 5 seconds:
# .github/workflows/speedreview.yml
- uses: jgravelle/jcodemunch-mcp/speedreview@main
with:
groq_api_key: ${{ secrets.GROQ_API_KEY }}
See speedreview/README.md for full setup and configuration.
Ask any question about any codebase. Get an answer in under 3 seconds.
pip install jcodemunch-mcp[groq]
export GROQ_API_KEY=gsk_...
# Ask about a GitHub repo (auto-indexes on first use)
gcm "how does authentication work?" --repo pallets/flask
# Ask about the current directory
gcm "where are the API routes defined?"
# Interactive chat mode
gcm --chat --repo facebook/react
# Use the fast 8B model
gcm "what does parse_file do?" --fast
Combines jCodeMunch's token-efficient retrieval (BM25 + PageRank) with Groq's 280+ tok/s inference for near-instant answers. See gcm --help
for all options.
Speak a question, hear the answer. Full audio loop: Whisper STT → retrieval → LLM → Orpheus TTS.
pip install jcodemunch-mcp[groq-voice]
# Voice conversation with a codebase
gcm --voice --repo pallets/flask
# Press Enter to start recording, Enter again to stop
# Or type a question directly as text fallback
Push-to-talk via Enter key. Caps answers to ~100 words for natural spoken delivery. Requires a microphone.
Generate a narrated explainer video for any codebase in a single command.
pip install jcodemunch-mcp[groq-explain]
# Generate a 60-second narrated explainer
gcm explain --repo pallets/flask -o flask-explainer.mp4
# With verbose timing
gcm explain --repo facebook/react -v
Pipeline: repo structure → LLM narration script → Orpheus TTS → Pillow slides → FFmpeg MP4. Requires FFmpeg on PATH.
Settings are controlled by a JSONC config file (config.jsonc
) with env var fallbacks for backward compatibility. Defaults are chosen so that a fresh install works without any configuration.
jcodemunch-mcp config --init # create ~/.code-index/config.jsonc from template
jcodemunch-mcp config # show effective configuration
jcodemunch-mcp config --check # validate config + verify prerequisites
--check
validates that your config file is well-formed, your AI provider package is installed, your index storage path is writable, and HTTP transport packages are present. Exits non-zero on any failure — useful for CI/CD or first-run scripts.
| Layer | Path | Purpose |
|---|---|---|
| Global | ~/.code-index/config.jsonc |
Server-wide defaults |
| Project | {project_root}/.jcodemunch.jsonc |
Per-project overrides |
Project config merges over global config — closest to the work wins.
| Config key | What it controls | Typical savings |
|---|---|---|
tool_profile |
"core" (16 tools), "standard" (51), "full" (62, default) |
~5-6k tokens (core) |
compact_schemas |
Strip rarely-used advanced params from schemas | ~1-2k tokens |
disabled_tools |
Remove individual tools from schema entirely | ~100–400 tokens/tool |
languages |
Shrink language enum + gate features | ~2–86 tokens/turn |
meta_fields |
Filter _meta response fields |
~50–150 tokens/call |
descriptions |
Control description verbosity | ~0–600 tokens/turn |
Recommended for context-conscious setups: "tool_profile": "core", "compact_schemas": true
reduces the schema footprint from ~11.5k tokens to ~4k tokens.
See the full template for all available keys. Run jcodemunch-mcp config --init
to generate one.
jcodemunch-mcp exposes 60+ tools. On request-capped plans, having all of them visible to small models causes primitive-preference bias (many search → read → search → read
cycles instead of one get_context_bundle
). The server mitigates this by narrowing the exposed tool list per the running model.
Three tiers ship with sensible defaults, fully editable in config.jsonc
:
core
(16 tools): indexing, search, retrieval. Recommended for Haiku / small local models.standard
(51 tools): core + analytics / architecture / quality. Recommended for Sonnet / GPT-4o class.full
(all 62 tools): no filter. Recommended for Opus / o1 / frontier models.
Edit tool_tier_bundles.core
/ tool_tier_bundles.standard
in your config.jsonc
to add or remove tools from each tier.
Runtime tier switching is off by default. To enable it, set in config.jsonc
:
When on, plan_turn
— already the opening-move tool — accepts an optional model
parameter that switches the session tier as a side effect, with no extra MCP request:
plan_turn(repo="...", query="...", model="claude-haiku-4-5")
The server resolves the model to a tier via model_tier_map
in config (fuzzy matching: normalizes the id, then exact → glob → substring → *
→ full
fallback). Subsequent tools/list
calls return only the narrowed set.
When adaptive_tiering
is false, plan_turn(model=...)
and announce_model(...)
accept their arguments but do not switch the tier — the static tool_profile
continues to drive the exposed tools. set_tool_tier(tier=...)
remains honored either way because it's an explicit user call, not automatic behavior.
disabled_tools
applies after tier filtering. A tool listed in both a tier bundle and disabled_tools
will not be exposed. The server logs a WARNING
on startup and jcodemunch-mcp config --check
prints a WARN:
row if this happens.
Place a .jcodemunch.jsonc
file at your project root to declare the layers your architecture must respect. get_layer_violations
will then enforce that imports only flow in the declared direction.
// .jcodemunch.jsonc — example for a layered Python project
{
"architecture": {
"layers": [
{ "name": "api", "paths": ["src/routes", "src/controllers"] },
{ "name": "service", "paths": ["src/services"] },
{ "name": "repo", "paths": ["src/repositories"] },
{ "name": "db", "paths": ["src/models", "src/migrations"] }
],
"rules": [
{ "layer": "api", "may_not_import": ["db"] },
{ "layer": "service", "may_not_import": ["api"] },
{ "layer": "repo", "may_not_import": ["api", "service"] }
]
}
}
Call get_layer_violations(rules=[...])
directly to pass rules inline — the config file is optional and used as a fallback. When no config is present, get_layer_violations
infers layers from top-level directory structure.
The following env vars still work but are deprecated. Config file values take priority:
| Variable | Config key | Default |
|---|---|---|
JCODEMUNCH_USE_AI_SUMMARIES |
use_ai_summaries |
true |
JCODEMUNCH_TRUSTED_FOLDERS |
trusted_folders |
[] |
JCODEMUNCH_MAX_FOLDER_FILES |
max_folder_files |
2000 |
JCODEMUNCH_MAX_INDEX_FILES |
max_index_files |
10000 |
JCODEMUNCH_STALENESS_DAYS |
staleness_days |
7 |
JCODEMUNCH_MAX_RESULTS |
max_results |
500 |
JCODEMUNCH_EXTRA_IGNORE_PATTERNS |
extra_ignore_patterns |
[] |
JCODEMUNCH_CONTEXT_PROVIDERS |
context_providers |
true |
JCODEMUNCH_REDACT_SOURCE_ROOT |
redact_source_root |
false |
JCODEMUNCH_STATS_FILE_INTERVAL |
stats_file_interval |
3 |
JCODEMUNCH_SHARE_SAVINGS |
share_savings |
true |
JCODEMUNCH_SUMMARIZER_CONCURRENCY |
summarizer_concurrency |
4 |
JCODEMUNCH_ALLOW_REMOTE_SUMMARIZER |
allow_remote_summarizer |
false |
JCODEMUNCH_RATE_LIMIT |
rate_limit |
0 |
JCODEMUNCH_TRANSPORT |
transport |
stdio |
JCODEMUNCH_HOST |
host |
127.0.0.1 |
JCODEMUNCH_PORT |
port |
8901 |
JCODEMUNCH_LOG_LEVEL |
log_level |
WARNING |
AI provider keys (ANTHROPIC_API_KEY
, GOOGLE_API_KEY
, OPENAI_API_BASE
, MINIMAX_API_KEY
, ZHIPUAI_API_KEY
, etc.), JCODEMUNCH_SUMMARIZER_PROVIDER
, and CODE_INDEX_PATH
are always read from env vars — they are never placed in config files.
AI provider priority in auto-detect mode: Anthropic → Gemini → OpenAI-compatible (OPENAI_API_BASE
) → MiniMax → GLM-5 → signature fallback. Set JCODEMUNCH_SUMMARIZER_PROVIDER
to force anthropic
, gemini
, openai
, minimax
, glm
, or none
. jcodemunch-mcp config
shows which provider is active.
allow_remote_summarizer
only affects OpenAI-compatible HTTP endpoints. When false
, jcodemunch accepts only localhost-style endpoints such as Ollama or LM Studio on 127.0.0.1
and rejects remote hosts like api.minimax.io
. When a remote endpoint is rejected, AI summarization falls back to docstrings or signatures instead of sending source code to that provider. Set allow_remote_summarizer: true
in config.jsonc
if you intentionally want to use a hosted OpenAI-compatible provider such as MiniMax or GLM-5.
A common question: does this only help during exploration, or also when the agent is prompted to read a file before editing?
It helps most when editing a specific function. The "read before edit" constraint doesn't require reading the whole file — it requires reading the code. get_symbol_source
gives you exactly the function body you're about to touch, nothing else. Instead of reading 700 lines to edit one method, you read those 30 lines.
| Scenario | Native tool | jCodemunch | Savings |
|---|---|---|---|
| Edit one function (700-line file) | Read → 700 lines |
get_symbol_source → 30 lines |
~95% |
| Understand a file's structure | Read → full content |
get_file_outline → names + signatures |
~80% |
| Find which file to edit | Grep many files |
search_symbols → exact match |
comparable |
| Edit requires whole-file context | Read → full content |
get_file_content → full content |
~0% |
| "What breaks if I change X?" | not possible | get_blast_radius |
unique capability |
The cases where it doesn't help: edits that genuinely require understanding the entire file (restructuring file-level state, reordering logic that spans hundreds of lines). For those, get_file_content
is roughly equivalent to Read
. The cases where it helps most are targeted edits — one function, one method, one class — which is the majority of real editing work.
- large repositories
- unfamiliar codebases
- agent-driven code exploration
- refactoring and impact analysis
- teams trying to cut AI token costs without making agents dumber
- developers who are tired of paying premium rates for glorified file scrolling
Start with QUICKSTART.md for the fastest setup path.
Then index a repo, ask your agent what it has indexed, and have it retrieve code by symbol instead of reading entire files. That is where the savings start.
jCodeMunch plugs into any MCP-compatible agent or IDE. Tested configurations:
| Platform | Config |
|---|---|
| Claude Code / Claude Desktop | jcodemunch-mcp init (auto-detects and patches config) |
| Cursor / Windsurf | jcodemunch-mcp init or manual mcp.json |
| Hermes Agent | Add to ~/.hermes/config.yaml — see skill |
| Any MCP client | stdio: jcodemunch-mcp , HTTP: jcodemunch-mcp serve --transport sse |
Hermes Agent config
# ~/.hermes/config.yaml
mcp_servers:
jcodemunch:
command: "uvx"
args: ["jcodemunch-mcp"]

---

# I built an MCP server that connects your code agent to Google AI Mode for free, token-efficient web research with citations - Reddit
Source URL: https://www.reddit.com/r/vibecoding/comments/1q6mtp0/i_built_an_mcp_server_that_connects_your_code/
Source Type: web_page
Source ID: fb7b922d-d87e-4ef7-be3d-58a3f7b44401

{
  "value": {
    "content": "I built an MCP server that connects your code agent to Google AI Mode for free, token-efficient web research with citations : r/vibecoding\n\nSkip to main content\n\nhttps://www.reddit.com/r/vibecoding/comments/1q6mtp0/i_built_an_mcp_server_that_connects_your_code/#main-content\n\n I built an MCP server that connects your code agent to Google AI Mode for free, token-efficient web research with citations : r/vibecoding\n\nOpen menu\n\nOpen navigation \n\nhttps://www.reddit.com/\n\nGo to Reddit Home\n\n \n\nr/vibecoding\n\nTRENDING TODAY\n\nGet App\n\nGet the Reddit app\n\nLog In\n\nhttps://www.reddit.com/login/\n\nLog in to Reddit\n\nExpand user menu\n\nOpen settings menu\n\nSkip to Navigation\n\nhttps://www.reddit.com/r/vibecoding/comments/1q6mtp0/i_built_an_mcp_server_that_connects_your_code/#left-sidebar-container\n\n \n\nSkip to Right Sidebar\n\nhttps://www.reddit.com/r/vibecoding/comments/1q6mtp0/i_built_an_mcp_server_that_connects_your_code/#right-sidebar-container\n\nBack\n\nGo to vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n•\n\n3mo ago\n\nPleasePrompto\n\nhttps://www.reddit.com/user/PleasePrompto/\n\nLocked post\n\nStickied post\n\nArchived post\n\nReport\n\nI built an MCP server that connects your code agent to Google AI Mode for free, token-efficient web research with citations\n\nA few days ago I got tired of watching Claude burn tokens reading 5-10 web pages just to answer a simple question about a library. So I built this skill that lets Google do the heavy lifting instead. Furthermore, I find the web research skills of all agents to be only “average”... to put it nicely.\n\nWhat it does\n\n: Your agent asks a question → Google AI Mode searches and synthesizes dozens of sources → your agent gets one clean answer with inline citations [1][2][3] → minimal token usage.\n\nWhy I built this\n\n: Traditional web research with code agents is expensive and slow:\n\nAgent searches Google → gets 10 links\n\nAgent reads 5-10 full pages → thousands of tokens consumed\n\nAgent synthesizes manually → risks missing details or hallucinating\n\nYou pay for all those tokens\n\nWith this server, Google does the synthesis. Your agent gets one structured answer with sources for maybe 1/10th of the tokens. And it's completely free.\n\nGitHub\n\n: \n\nhttps://github.com/PleasePrompto/google-ai-mode-mcp\n\nhttps://github.com/PleasePrompto/google-ai-mode-mcp\n\n \n\nnpm\n\n: \n\nnpx google-ai-mode-mcp@latest\n\nInstallation\n\n (works with Cursor, Codex, Cline, Windsurf, etc.):\n\n{\n  \"mcpServers\": {\n    \"google-ai-search\": {\n      \"command\": \"npx\",\n      \"args\": [\"google-ai-mode-mcp@latest\"]\n    }\n  }\n}\n\n\nOr use the CLI shortcuts:\n\ncodex mcp add google-ai-search -- npx google-ai-mode-mcp@latest\n# or: cursor, cline, etc.\n\n\nSimple usage\n\n:\n\nAsk your agent: \"Search Google AI Mode for: Next.js 15 App Router best practices\"\n\nGoogle reads and synthesizes 20+ sources automatically\n\nYour agent gets one clean answer with citations\n\nUse the cited sources to verify details\n\nReal example\n\n: I was implementing OAuth2 in Hono (a framework I'd never used). Instead of having my agent read through docs and blog posts:\n\nMe: \"Search for: Hono OAuth2 implementation guide 2026\"\nAgent: [calls MCP server]\nGoogle: [synthesizes 15+ sources]\nAgent: \"Here's the approach with code examples [1][2][3]...\"\n\n\nFirst implementation worked. Zero hallucinated APIs. All sources linked for verification.\n\nWhy this is better than regular web search:\n\nMethod\n\nToken Cost\n\nHallucinations\n\nResult\n\nAgent reads 5-10 pages\n\nVery high\n\nMedium - fills gaps\n\nWorking but expensive\n\nWeb research tools\n\nHigh\n\nHigh\n\nOutdated/unreliable\n\nGoogle AI Mode MCP\n\nMinimal\n\nLow - cites sources\n\nClean, grounded answers\n\nGoogle AI Mode isn't just search - it reads and analyzes dozens of websites, synthesizes findings, and cites every claim. Your agent gets the benefits without doing the work.\n\nImportant - Test Phase\n\n: I've only tested this on \n\nLinux\n\n with \n\nClaude Code, Codex, and Gemini\n\n (all CLI tools). It should work on Windows/Mac but I haven't tested those yet.\n\nCAPTCHA handling\n\n: First time you use it, Google might show a CAPTCHA. Just tell your agent to show the browser, solve it once, and you're good to go. The server uses a persistent browser profile to minimize future CAPTCHAs.\n\nLinux/WSL users on Codex\n\n: If you get \"Missing X-Server\" errors, wrap with xvfb-run:\n\n{\n  \"mcpServers\": {\n    \"google-ai-search\": {\n      \"command\": \"xvfb-run\",\n      \"args\": [\"-a\", \"npx\", \"google-ai-mode-mcp@latest\"]\n    }\n  }\n}\n\n\nBuilt this for myself but figured others might be tired of expensive web research too. Questions welcome!\n\nFor Claude Code users\n\n: There's also a lightweight skill version that doesn't require MCP server setup: \n\nhttps://github.com/PleasePrompto/google-ai-mode-skill\n\nhttps://github.com/PleasePrompto/google-ai-mode-skill\n\nUpvote 3 Downvote 2 Go to comments Share\n\nSort by: Best\n\nOpen comment sort options\n\nBest\n\nTop\n\nNew\n\nControversial\n\nOld\n\nQ&A\n\nSearch Comments Expand comment search\n\nCancel\n\nComments Section\n\nPleasePrompto\n\nhttps://www.reddit.com/user/PleasePrompto/\n\nOP\n\n• \n\n3mo ago\n\nhttps://www.reddit.com/r/vibecoding/comments/1q6mtp0/comment/nycm9gc/\n\nI made the MCP server / skill a lot more robust this morning in terms of multilingual detection (I now consistently use the Thumbs Up button as the FIRST indicator!). When the Thumbs Up button appears, it signals > Answer is ready. This way, I get around the problem of a browser running in Arabic, for example, where the Google interface is in Arabic and the MCP doesn't know: the response is ready! I've also added a few other long indicators as a fallback, and if nothing works, whatever is there will be taken after 40 seconds at the latest!\n\nMy tests here locally were successful.\n\nIt should work much better now, so feel free to pull/update and test!\n\nUpvote 1 Downvote Reply Award Share\n\nReport\n\nAward\n\nShare \n\nLow_Variation5730\n\nhttps://www.reddit.com/user/Low_Variation5730/\n\n•\n\n2mo ago\n\nhttps://www.reddit.com/r/vibecoding/comments/1q6mtp0/comment/o668gmp/\n\nGreat work bro really helped me. But another thought is when I'm running this on a vps, wht to do about captcha?\n\nUpvote 1 Downvote Reply Award Share\n\nReport\n\nAward\n\nShare\n\nNew to Reddit?\n\nCreate your account and connect with a world of communities.\n\nContinue with Email\n\nhttps://www.reddit.com/register/\n\nContinue With Phone Number\n\nhttps://www.reddit.com/login/\n\nBy continuing, you agree to our \n\nUser Agreement\n\nhttps://www.redditinc.com/policies/user-agreement\n\n and acknowledge that you understand the \n\nPrivacy Policy\n\nhttps://www.redditinc.com/policies/privacy-policy\n\n.\n\nRelated Answers Section\n\nRelated Answers\n\nInnovative AI tools for creative coding\n\nhttps://www.reddit.com/answers/16f50785-8995-4209-8f27-7535ce6f486a/?q=Innovative+AI+tools+for+creative+coding&source=PDP\n\nUnique programming languages to explore\n\nhttps://www.reddit.com/answers/f085d69b-1a2c-4926-becc-44dd6f8d3660/?q=Unique+programming+languages+to+explore&source=PDP\n\nHow to enhance coding with music\n\nhttps://www.reddit.com/answers/8a68eca6-5be6-4577-90fb-4da96fdbc8cc/?q=How+to+enhance+coding+with+music&source=PDP\n\nBest practices for vibe coding techniques\n\nhttps://www.reddit.com/answers/d3046a6b-c1f2-4257-ae93-9a99ca72ed7e/?q=Best+practices+for+vibe+coding+techniques&source=PDP\n\nAI-generated art: tools and tips\n\nhttps://www.reddit.com/answers/48444d04-c31f-4389-a63f-df7754a348d4/?q=AI-generated+art%3A+tools+and+tips&source=PDP\n\nMore posts you may like\n\nI built an MCP server that gives coding agents access to 2M research papers. Tested it with autoresearch - here's what happened.\n\nhttps://www.reddit.com/r/LLMDevs/comments/1s6izfu/i_built_an_mcp_server_that_gives_coding_agents/\n\n \n\nr/LLMDevs\n\nhttps://www.reddit.com/r/LLMDevs/\n\n • 11d ago [\n\nI built an MCP server that gives coding agents access to 2M research papers. Tested it with autoresearch - here's what happened.\n\n](https://www.reddit.com/r/LLMDevs/comments/1s6izfu/i_built_an_mcp_server_that_gives_coding_agents/) \n\n 4 147 upvotes · 31 comments\n\nI built an MCP that finally makes your AI agents shine with SQL\n\nhttps://www.reddit.com/r/AI_Agents/comments/1llwqqb/i_built_an_mcp_that_finally_makes_your_ai_agents/\n\n \n\nr/AI_Agents\n\nhttps://www.reddit.com/r/AI_Agents/\n\n • 10mo ago [\n\nI built an MCP that finally makes your AI agents shine with SQL\n\n](https://www.reddit.com/r/AI_Agents/comments/1llwqqb/i_built_an_mcp_that_finally_makes_your_ai_agents/) 33 upvotes · 34 comments\n\nBuilt an MCP server that gives AI agents a full codebase map instead of reading files one at a time\n\nhttps://www.reddit.com/r/mcp/comments/1rjdoag/built_an_mcp_server_that_gives_ai_agents_a_full/\n\n \n\nr/mcp\n\nhttps://www.reddit.com/r/mcp/\n\n • 1mo ago [\n\nBuilt an MCP server that gives AI agents a full codebase map instead of reading files one at a time\n\n](https://www.reddit.com/r/mcp/comments/1rjdoag/built_an_mcp_server_that_gives_ai_agents_a_full/) 54 upvotes · 44 comments\n\nAn MCP to improve your coding agent with better memory using code indexing and accurate semantic search\n\nhttps://www.reddit.com/r/LocalLLaMA/comments/1oa1gz9/an_mcp_to_improve_your_coding_agent_with_better/\n\n \n\nr/LocalLLaMA\n\nhttps://www.reddit.com/r/LocalLLaMA/\n\n • 6mo ago [\n\nAn MCP to improve your coding agent with better memory using code indexing and accurate semantic search\n\n](https://www.reddit.com/r/LocalLLaMA/comments/1oa1gz9/an_mcp_to_improve_your_coding_agent_with_better/) 17 upvotes · 12 comments\n\nAny Cheap AI model for coding?\n\nhttps://www.reddit.com/r/vibecoding/comments/1pp4z2l/any_cheap_ai_model_for_coding/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 4mo ago [\n\nAny Cheap AI model for coding?\n\n](https://www.reddit.com/r/vibecoding/comments/1pp4z2l/any_cheap_ai_model_for_coding/) 11 upvotes · 52 comments\n\nI built a Local MCP Server to enable Computer-Use Agent to run through Claude Desktop, Cursor, and other MCP clients.\n\nhttps://www.reddit.com/r/LocalLLaMA/comments/1k3a3kl/i_built_a_local_mcp_server_to_enable_computeruse/\n\n \n\nr/LocalLLaMA\n\nhttps://www.reddit.com/r/LocalLLaMA/\n\n • 1y ago [\n\nI built a Local MCP Server to enable Computer-Use Agent to run through Claude Desktop, Cursor, and other MCP clients.\n\n](https://www.reddit.com/r/LocalLLaMA/comments/1k3a3kl/i_built_a_local_mcp_server_to_enable_computeruse/) \n\n 1:55 43 upvotes · 7 comments\n\nFinally, a GUI Tool for Managing MCP Servers Across AI Agents!\n\nhttps://www.reddit.com/r/mcp/comments/1otcloj/finally_a_gui_tool_for_managing_mcp_servers/\n\n \n\nr/mcp\n\nhttps://www.reddit.com/r/mcp/\n\n • 5mo ago [\n\nFinally, a GUI Tool for Managing MCP Servers Across AI Agents!\n\n](https://www.reddit.com/r/mcp/comments/1otcloj/finally_a_gui_tool_for_managing_mcp_servers/) \n\n 37 upvotes · 27 comments\n\nBest AI for programming + general use that you guys use?\n\nhttps://www.reddit.com/r/vibecoding/comments/1rqdk78/best_ai_for_programming_general_use_that_you_guys/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 1mo ago [\n\nBest AI for programming + general use that you guys use?\n\n](https://www.reddit.com/r/vibecoding/comments/1rqdk78/best_ai_for_programming_general_use_that_you_guys/) 1 upvote · 25 comments\n\nI built a VS Code extension that turns your Claude Code agents into pixel art characters working in a little office | Free & Open-source\n\nhttps://www.reddit.com/r/vibecoding/comments/1rbs3dq/i_built_a_vs_code_extension_that_turns_your/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 2mo ago [\n\nI built a VS Code extension that turns your Claude Code agents into pixel art characters working in a little office | Free & Open-source\n\n](https://www.reddit.com/r/vibecoding/comments/1rbs3dq/i_built_a_vs_code_extension_that_turns_your/) \n\n 0:53 271 upvotes · 25 comments\n\nI built an open-source MCP server that lets any Agent work on remote machines\n\nhttps://www.reddit.com/r/mcp/comments/1rhyeme/i_built_an_opensource_mcp_server_that_lets_any/\n\n \n\nr/mcp\n\nhttps://www.reddit.com/r/mcp/\n\n • 1mo ago [\n\nI built an open-source MCP server that lets any Agent work on remote machines\n\n](https://www.reddit.com/r/mcp/comments/1rhyeme/i_built_an_opensource_mcp_server_that_lets_any/) \n\n 0:50 26 upvotes · 7 comments\n\nAI Agents, Sub-Agents, Skills, MCP, and a parallel with a traditional corporate organization.\n\nhttps://www.reddit.com/r/vibecoding/comments/1qur9xt/ai_agents_subagents_skills_mcp_and_a_parallel/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 2mo ago [\n\nAI Agents, Sub-Agents, Skills, MCP, and a parallel with a traditional corporate organization.\n\n](https://www.reddit.com/r/vibecoding/comments/1qur9xt/ai_agents_subagents_skills_mcp_and_a_parallel/) 5 upvotes · 9 comments\n\nI built a stable full-stack app with MCP-connected Claude Code to manage the backend.\n\nhttps://www.reddit.com/r/vibecoding/comments/1rpzzmi/i_built_a_stable_fullstack_app_with_mcpconnected/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 1mo ago [\n\nI built a stable full-stack app with MCP-connected Claude Code to manage the backend.\n\n](https://www.reddit.com/r/vibecoding/comments/1rpzzmi/i_built_a_stable_fullstack_app_with_mcpconnected/) 8 upvotes · 3 comments\n\nI built a local AI agent that turns my messy computer into a private, searchable memory\n\nhttps://www.reddit.com/r/PKMS/comments/1nfa8hk/i_built_a_local_ai_agent_that_turns_my_messy/\n\n \n\nr/PKMS\n\nhttps://www.reddit.com/r/PKMS/\n\n • 7mo ago [\n\nI built a local AI agent that turns my messy computer into a private, searchable memory\n\n](https://www.reddit.com/r/PKMS/comments/1nfa8hk/i_built_a_local_ai_agent_that_turns_my_messy/) \n\n 121 upvotes · 50 comments\n\nI built a CLI tool to standardize your AI coding agent workflows (Claude Code, Cursor, Copilot, Gemini, etc.) with a single command\n\nhttps://www.reddit.com/r/vibecoding/comments/1rn0caj/i_built_a_cli_tool_to_standardize_your_ai_coding/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 1mo ago [\n\nI built a CLI tool to standardize your AI coding agent workflows (Claude Code, Cursor, Copilot, Gemini, etc.) with a single command\n\n](https://www.reddit.com/r/vibecoding/comments/1rn0caj/i_built_a_cli_tool_to_standardize_your_ai_coding/) 9 upvotes · 4 comments\n\nI made this CLI and MCP that saves 50-99% of tokens and runs local on your machine.\n\nhttps://www.reddit.com/r/vibecoding/comments/1oa01c6/i_made_this_cli_and_mcp_that_saves_5099_of_tokens/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 6mo ago [\n\nI made this CLI and MCP that saves 50-99% of tokens and runs local on your machine.\n\n](https://www.reddit.com/r/vibecoding/comments/1oa01c6/i_made_this_cli_and_mcp_that_saves_5099_of_tokens/) 19 upvotes · 35 comments\n\nI've compiled a list of all AI related Tools & Resources in a single place.\n\nhttps://www.reddit.com/r/vibecoding/comments/1nsk6ys/ive_compiled_a_list_of_all_ai_related_tools/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 6mo ago [\n\nI've compiled a list of all AI related Tools & Resources in a single place.\n\n](https://www.reddit.com/r/vibecoding/comments/1nsk6ys/ive_compiled_a_list_of_all_ai_related_tools/) 7 upvotes · 5 comments\n\n11 months of AI coding - my experience (long post with screenshots)\n\nhttps://www.reddit.com/r/vibecoding/comments/1o3u8iz/11_months_of_ai_coding_my_experience_long_post/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 6mo ago [\n\n11 months of AI coding - my experience (long post with screenshots)\n\n](https://www.reddit.com/r/vibecoding/comments/1o3u8iz/11_months_of_ai_coding_my_experience_long_post/) \n\n 87 upvotes · 47 comments\n\nUsing multiple AI models together was a game changer for my side projects\n\nhttps://www.reddit.com/r/vibecoding/comments/1q2lr9a/using_multiple_ai_models_together_was_a_game/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 3mo ago [\n\nUsing multiple AI models together was a game changer for my side projects\n\n](https://www.reddit.com/r/vibecoding/comments/1q2lr9a/using_multiple_ai_models_together_was_a_game/) 15 upvotes · 10 comments\n\nJust discovered an insane open-source multi-agent coding CLI\n\nhttps://www.reddit.com/r/vibecoding/comments/1o1ssrc/just_discovered_an_insane_opensource_multiagent/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 6mo ago [\n\nJust discovered an insane open-source multi-agent coding CLI\n\n](https://www.reddit.com/r/vibecoding/comments/1o1ssrc/just_discovered_an_insane_opensource_multiagent/) 7 upvotes · 25 comments\n\nI built a \"Memory API\" to give AI agents long-term context (Open Source & Hosted)\n\nhttps://www.reddit.com/r/VibeCodersNest/comments/1paktkp/i_built_a_memory_api_to_give_ai_agents_longterm/\n\n \n\nr/VibeCodersNest\n\nhttps://www.reddit.com/r/VibeCodersNest/\n\n • 4mo ago [\n\nI built a \"Memory API\" to give AI agents long-term context (Open Source & Hosted)\n\n](https://www.reddit.com/r/VibeCodersNest/comments/1paktkp/i_built_a_memory_api_to_give_ai_agents_longterm/) 29 upvotes · 13 comments\n\nAre you coding 100% with AI, and if you are from non-tech background, i would love you\n\nhttps://www.reddit.com/r/vibecoding/comments/1rcb09t/are_you_coding_100_with_ai_and_if_you_are_from/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 2mo ago [\n\nAre you coding 100% with AI, and if you are from non-tech background, i would love you\n\n](https://www.reddit.com/r/vibecoding/comments/1rcb09t/are_you_coding_100_with_ai_and_if_you_are_from/) 3 upvotes · 32 comments\n\nI built an open-source AI chat that renders responses as actual UI components (charts, tables, etc.) instead of just markdown\n\nhttps://www.reddit.com/r/vibecoding/comments/1r83fqx/i_built_an_opensource_ai_chat_that_renders/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 2mo ago [\n\nI built an open-source AI chat that renders responses as actual UI components (charts, tables, etc.) instead of just markdown\n\n](https://www.reddit.com/r/vibecoding/comments/1r83fqx/i_built_an_opensource_ai_chat_that_renders/) 52 upvotes · 11 comments\n\nI built & publicly host a handful of MCP servers - free to use, no API keys/auth needed\n\nhttps://www.reddit.com/r/ClaudeAI/comments/1sceak4/i_built_publicly_host_a_handful_of_mcp_servers/\n\n \n\nr/ClaudeAI\n\nhttps://www.reddit.com/r/ClaudeAI/\n\n • 5d ago [\n\nI built & publicly host a handful of MCP servers - free to use, no API keys/auth needed\n\n](https://www.reddit.com/r/ClaudeAI/comments/1sceak4/i_built_publicly_host_a_handful_of_mcp_servers/) \n\n 46 upvotes · 11 comments\n\nawesome-opensource-ai - Curated list of the best truly open-source AI projects, models, tools, and infrastructure\n\nhttps://www.reddit.com/r/vibecoding/comments/1scz37x/awesomeopensourceai_curated_list_of_the_best/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 4d ago [\n\nawesome-opensource-ai - Curated list of the best truly open-source AI projects, models, tools, and infrastructure\n\n](https://www.reddit.com/r/vibecoding/comments/1scz37x/awesomeopensourceai_curated_list_of_the_best/) \n\n 106 upvotes · 18 comments\n\nMy honest take on AI coding tools after using them daily for 2 years as a developer\n\nhttps://www.reddit.com/r/vibecoding/comments/1s8mwz6/my_honest_take_on_ai_coding_tools_after_using/\n\n \n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n • 9d ago [\n\nMy honest take on AI coding tools after using them daily for 2 years as a developer\n\n](https://www.reddit.com/r/vibecoding/comments/1s8mwz6/my_honest_take_on_ai_coding_tools_after_using/) 11 comments\n\nCommunity Info Section\n\nr/vibecoding\n\nhttps://www.reddit.com/r/vibecoding/\n\n \n\n \n\nVibeJam is happening!\n\nJoin\n\nvibecoding\n\nfully give in to the vibes. forget that the code even exists.\n\nShow more\n\nPublic\n\nAnyone can view, post, and comment to this community\n\nReddit Rules\n\nhttps://www.redditinc.com/policies/content-policy\n\n \n\nPrivacy Policy\n\nhttps://www.reddit.com/policies/privacy-policy\n\n \n\nUser Agreement\n\nhttps://www.redditinc.com/policies/user-agreement\n\n \n\nYour Privacy Choices\n\nhttps://support.reddithelp.com/hc/articles/43980704794004\n\n \n\nAccessibility\n\nhttps://support.reddithelp.com/hc/sections/38303584022676-Accessibility\n\n \n\nReddit, Inc. © 2026. All rights reserved.\n\nhttps://redditinc.com/\n\nExpand Navigation\n\nExpand Navigation\n\nCollapse Navigation\n\nCollapse Navigation\n\n \n\n0cAFcWeA4sh51KdhD_TGD6VxpO7jsh96GQeZ8A2ZcTB-a8Rl6ng8MAl5StZ9ckVq68Dx4oC3lkIkTusF0DqXH2Shf27md9MeTh9BWz6WFyi8-hsFI-qh_LKWDQrJElCmKpRyErIPYuDJ5Tlx2VoVKiGIPRDL15juF_lCUbHWLiWFrkFYYx7htgyVTesGbE-tUp8AU97VH0xCwTGdj6I4Suh0peKwkf6pcCkVCKSmm3ikTBNeLeFKN0uinvvQIVTYvWADrVQnR7CWLy37Vf5nJuwuEz9gqlykNryA5PJUrZcYAu09flThBoqu4jfXimiY3VvB3KfR4DBfJvxn9KTlgWccFQ2D4jAYJL1cMU-79YSi-jeK8y2zgQ2VPp_AML5tdR6TqlE1gLGS9BQag-tzp4Xbcgw5dl7xQTBUGxopJ2Pxl9-HrHXWoYpwprwvekMqh9T0ycwwT7mI-2gTDf6rPyrCfqKDhHn-hdYXijnt5UNfu6Z7ydo3WB7A-27q8Wdq8raQd9rFcsQgdVhtjyCSiE6Y0vPo0jCPeffPRd6JFiNG4k1_Cq6jnBRdN6V4cg-Iz1TcKWqF4xXo158EFa97GrNuWssHo9F3OCt5pGNyz_FJCPQ9aCMgRZkrNZ_GScIwhBe8UUpq-YG7LMq8jRafz58qxNufjltlnYPJXrFO2-bhk2hNGI31BYEnC31NIvigiPSVsHB1O1dIr5Oibvp4W8yeAZuPilElYQLuVNsZqTht4UNnD43Tqv4RAkcEQn5HZh388Qx1BymvWlWCBh9Rjn1x50XA2VPOa97ksFdnmgA4nvaE2Bc8n5JPi4rgL7SxBJck-nJaGL3FllZzL2iY9iAuHBMoq8B4_oYaYgeyOep5AChxjOeuezv0FmC0el0yezCxfvbHz7MBWhTPjTLUV2-aznqpyTmIs2DnkN1wPzfb3oGRDY60nhPqKiII0-xlifvs7QHgNYXEz4J40FAM8ZSEC0c0CstwogwjbOeDvNpZSwESki3WuUHt6n0O2HEaedEG_9vBgkWEVQdeiCWygdn4CSsIZUGtBrH1_MgaYt7FF5mkWC9qJApNWhRfwikFHu-1JwwCJapcxEilxUKHxXwOjtsLCzX76wnfBOWxNszaIF9wh4SNv-siqeNwSUcqcio5fBniSVSVW9SCFOzkuQvucD7Mjq0STRGST7WE79kwf7r0RH9VlKmmGnUGVKzZsp739gyi2aIRQxLdS9fk3tI46iUh9dH2MVk2Zj0LylSgof5RT4dti5t3ABK07-YyxEOg0zEc6Rb59E3FxvXfD1fWkABoZg8PPmWkRf9YuiQmyMQfjRhUtRNQoPNebcbGOBCd4pScfvK7paUy8FEGDn2rEjVuTStnEFyblU-WgwZP-SoD__Op1n7gMdxc1skOud_lvBHbzZeH-B3fYpeDH1S1k_CxcCLxFvJjeyFwpt451R1W-S4Oe19WScnyLiqqGvCnnvRVfNs0Vx0achHfk2OYf1QAG9ektEcsXvKJTFS-cG0k9vIgua0K8T6tjpueCCtgSVTxo8EQL8cUqmI9o8Wuc6YDgwe7aFYZoPt4f_BIkXVKzNcKHjONECkXh8f2pCy0QZasS0bTmn6IH9iMfqE9fHx5g85r_umgn9cbY5jD_wfY5NKzyPKWupGZGJbQzjPif-n2H-_ndq4k3GcTSmiSfpDZ26BlgRU59-VIE0mOlnA9lJb9UqImGW9TES66qkom7sOfgRPEB4ZKQ5tDMoGjFFLY8GH3fs2V1T5JNJloBXGqjFU69EINRzR8F3aplzdSQyNxrffhVutxCK7TEM5U0xuJ4PK9zvZ5EJclpgy6fa8bVHOA4uLRMcGKIQiJkwCofn_1buW-GT5oFz9uIh1rSzR3D63G5cr9WucpRL3eaZGUTjkwAhx6uQbNwLbk9Sxvp9FN17jLWXXpzo59-rYAQqoUyLmT2JkBrUBysD8HScN64awmDTo7gzk4nT0Eu3riB_lDkG82ZlUs8n6ppXK18NPqwZGyckihlbMlKE0LIA3S-JYyzy2UlRKFhJ7H827sX86oEd5G5Gk1f_q0tBdsTPTYqyeVeILjORSCs4jMDo4xGx_LePUjhSPdWykJ8lADuL32Zm5SoUHa5INdkcvIVIf2-oSvcn3fDegNwTCWEZw9p_RIfmuWgUGzxi_cb_6k53sNCBfqQa2lHxoleHmLNfHLJGDPyCTw",
    "title": "I built an MCP server that connects your code agent to Google AI Mode for free, token-efficient web research with citations - Reddit",
    "source_type": "web_page",
    "url": "https://www.reddit.com/r/vibecoding/comments/1q6mtp0/i_built_an_mcp_server_that_connects_your_code/",
    "char_count": 22344
  }
}


---

