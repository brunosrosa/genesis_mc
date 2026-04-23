# 🧠 SODA CANONICAL KNOWLEDGE BASE
> Gerado pelo @soda-knowledge-curator.
> Fontes Originais: 249 | Fontes Puras: 247 | Temas Identificados: 24



## 🧩 Eixo Temático 1

# mindfold-ai/Trellis: All-in-one AI framework & toolkit · GitHub - GitHub
Source URL: https://github.com/mindfold-ai/Trellis

Source Type: web_page

Source ID: 5ed185aa-1047-42ca-94a0-9d7cbe955b35


A multi-platform AI coding framework that rules
Supports Claude Code, Cursor, OpenCode, iFlow, Codex, Kilo, Kiro, Gemini CLI, Antigravity, Windsurf, Qoder, CodeBuddy, GitHub Copilot, and Factory Droid.
简体中文 • Docs • Quick Start • Supported Platforms • Use Cases
| Capability | What it changes |
|---|---|
| Auto-injected specs | Write conventions once in .trellis/spec/ , then let Trellis inject the relevant context into each session instead of repeating yourself. |
| Task-centered workflow | Keep PRDs, implementation context, review context, and task status in .trellis/tasks/ so AI work stays structured. |
| Parallel agent execution | Run multiple AI tasks side by side with git worktrees instead of turning one branch into a traffic jam. |
| Project memory | Journals in .trellis/workspace/ preserve what happened last time, so each new session starts with real context. |
| Team-shared standards | Specs live in the repo, so one person’s hard-won workflow or rule can benefit the whole team. |
| Multi-platform setup | Bring the same Trellis structure to 14 AI coding platforms instead of rebuilding your workflow per tool. |
- Node.js ≥ 18
- Python ≥ 3.10 (required for hooks and automation scripts)
# 1. Install Trellis
npm install -g @mindfoldhq/trellis@latest
# 2. Initialize in your repo
trellis init -u your-name
# 3. Or initialize with the platforms you actually use
trellis init --cursor --opencode --codex -u your-name
-u your-name
creates.trellis/workspace/your-name/
for personal journals and session continuity.- Platform flags can be mixed and matched. Current options include
--cursor
,--opencode
,--iflow
,--codex
,--kilo
,--kiro
,--gemini
,--antigravity
,--windsurf
,--qoder
,--codebuddy
,--copilot
, and--droid
.
| Command | What it does |
|---|---|
/start |
Load project context into the session. Run once at the beginning — on platforms with hooks (Claude Code, iFlow, OpenCode, Codex, GitHub Copilot) this happens automatically. |
/brainstorm |
Walk through requirements and produce a PRD. Use when starting a new feature or when the scope is unclear. |
/before-dev |
Read the relevant specs before you start coding (auto-detects frontend/backend). Run after /brainstorm and before writing any code. |
/check |
Review your changes against project specs and auto-fix violations (auto-detects frontend/backend). Run after implementation, before committing. |
/finish-work |
Pre-commit checklist covering lint, tests, docs, and API changes. Run right before git commit as a final gate. |
/parallel |
Spin up multiple agents in isolated git worktrees. Use for large tasks that can be split into independent subtasks. |
/record-session |
Save a session summary to the workspace journal. Run after the human has tested and committed the code. |
/update-spec |
Capture a new pattern or convention into spec files. Run whenever you discover a rule worth preserving for future sessions. |
You write a database naming rule once in .trellis/spec/backend/database-guidelines.md
. From that point on, every session — whether started by you, a teammate, or a parallel agent — gets that rule injected automatically. No more pasting the same instructions into every chat window.
Run /parallel
to spin up three agents, each in its own git worktree with its own branch. They implement, self-check, and open draft PRs independently. You review and merge when ready — no waiting, no branch conflicts.
When you /record-session
at the end of a workday, the current session summary lands in your workspace journal. When you start a new session the next day, the startup hook picks it up automatically, so the AI already knows what you shipped, what broke, and what’s left.
Run trellis init --cursor --claude
once. Both tools read the same .trellis/spec/
and .trellis/tasks/
. A spec improvement made in a Claude Code session is available in Cursor the next time someone opens the project.
Trellis keeps the core workflow in .trellis/
and generates the platform-specific entry points you need around it.
.trellis/
├── spec/ # Project standards, patterns, and guides
├── tasks/ # Task PRDs, context files, and status
├── workspace/ # Journals and developer-specific continuity
├── workflow.md # Shared workflow rules
└── scripts/ # Utilities that power the workflow
Depending on the platforms you enable, Trellis also creates tool-specific integration files such as .claude/
, .cursor/
, AGENTS.md
, .agents/
, .codex/
, .kilocode/
, .kiro/skills/
, .gemini/
, .agent/workflows/
(Antigravity), .windsurf/workflows/
, .qoder/
, .codebuddy/
, .github/copilot/
, .github/hooks/
, .github/prompts/
, and .factory/
(Droid). For Codex, Trellis also installs project skills under .agents/skills/
(shared with Cursor, Gemini CLI, GitHub Copilot, Amp, and Kimi Code).
At a high level, the workflow is simple:
- Define standards in specs.
- Start or refine work from a task PRD.
- Let Trellis inject the right context for the current task.
- Use checks, journals, and worktrees to keep quality and continuity intact.
Specs ship as empty templates by default — they are meant to be customized for your project's stack and conventions. You can fill them from scratch, or start from a community template:
# Fetch templates from a custom registry
trellis init --registry https://github.com/your-org/your-spec-templates
Browse available templates and learn how to publish your own on the Spec Templates page.
- v0.4.0: command consolidation (
before-backend-dev
+before-frontend-dev
→before-dev
,check-backend
+check-frontend
→check
), new/update-spec
command for capturing knowledge into specs, internal Python scripts refactoring. - v0.3.6: task lifecycle hooks, custom template registries (
--registry
), parent-child subtasks, fix PreToolUse hook for CC v2.1.63+. - v0.3.5: hotfix for delete migration manifest field name (Kilo workflows).
- v0.3.4: Qoder platform support, Kilo workflows migration, record-session task awareness.
- v0.3.1: background watch mode for
trellis update
, improved.gitignore
handling, docs refresh. - v0.3.0: platform support expanded from 2 to 10, Windows compatibility, remote spec templates,
/trellis:brainstorm
.
How is this different from CLAUDE.md
, AGENTS.md
, or .cursorrules
?
Those files are useful, but they tend to become monolithic. Trellis adds structure around them: layered specs, task context, workspace memory, and platform-aware workflow wiring.
Is Trellis only for Claude Code?
No. Trellis supports 14 platforms today. See the Supported Platforms guide for the full list and per-tool setup.
Do I have to write every spec file manually?
No. Many teams start by letting AI draft specs from existing code and then tighten the important parts by hand. Trellis works best when you keep the high-signal rules explicit and versioned.
Can teams use this without constant conflicts?
Yes. Personal workspace journals stay separate per developer, while shared specs and tasks stay in the repo where they can be reviewed and improved like any other project artifact.
- Official Docs - Product docs, setup guides, and architecture
- Quick Start - Get Trellis running in a repo fast
- Supported Platforms - Platform-specific setup and command details
- Real-World Scenarios - See how the workflow plays out in practice
- Changelog - Track current releases and updates
- Tech Blog - Product thinking and technical writeups
- GitHub Issues - Report bugs or request features
- Discord - Join the community
Official Repository • AGPL-3.0 License • Built by Mindfold

---

# GitHub - BloopAI/vibe-kanban: Get 10X more out of Claude Code, Codex or any coding agent
Source URL: https://github.com/BloopAI/vibe-kanban

Source Type: web_page

Source ID: ca266c8f-0269-4e40-82c8-7172303f60e4


Get 10X more out of Claude Code, Gemini CLI, Codex, Amp and other coding agents...
In a world where software engineers spend most of their time planning and reviewing coding agents, the most impactful way to ship more is to get faster at planning and review.
Vibe Kanban is built for this. Use kanban issues to plan work, either privately or with your team. When you're ready to begin, create workspaces where coding agents can execute.
- Plan with kanban issues — create, prioritise, and assign issues on a kanban board
- Run coding agents in workspaces — each workspace gives an agent a branch, a terminal, and a dev server
- Review diffs and leave inline comments — send feedback directly to the agent without leaving the UI
- Preview your app — built-in browser with devtools, inspect mode, and device emulation
- Switch between 10+ coding agents — Claude Code, Codex, Gemini CLI, GitHub Copilot, Amp, Cursor, OpenCode, Droid, CCR, and Qwen Code
- Create pull requests and merge — open PRs with AI-generated descriptions, review on GitHub, and merge
One command. Describe the work, review the diff, ship it.
npx vibe-kanban
Make sure you have authenticated with your favourite coding agent. A full list of supported coding agents can be found in the docs. Then in your terminal run:
npx vibe-kanban
Head to the website for the latest documentation and user guides.
Want to host your own Vibe Kanban Cloud instance? See our self-hosting guide.
We use GitHub Discussions for feature requests. Please open a discussion to create a feature request. For bugs please open an issue on this repo.
We would prefer that ideas and changes are first raised with the core team via GitHub Discussions or Discord, where we can discuss implementation details and alignment with the existing roadmap. Please do not open PRs without first discussing your proposal with the team.
Additional development tools:
cargo install cargo-watch
cargo install sqlx-cli
Install dependencies:
pnpm i
pnpm run dev
This will start the backend and web app. A blank DB will be copied from the dev_assets_seed
folder.
To build just the web app:
cd packages/local-web
pnpm run build
- Run
./local-build.sh
- Test with
cd npx-cli && node bin/cli.js
The following environment variables can be configured at build time or runtime:
| Variable | Type | Default | Description |
|---|---|---|---|
POSTHOG_API_KEY |
Build-time | Empty | PostHog analytics API key (disables analytics if empty) |
POSTHOG_API_ENDPOINT |
Build-time | Empty | PostHog analytics endpoint (disables analytics if empty) |
PORT |
Runtime | Auto-assign | Production: Server port. Dev: Frontend port (backend uses PORT+1) |
BACKEND_PORT |
Runtime | 0 (auto-assign) |
Backend server port (dev mode only, overrides PORT+1) |
FRONTEND_PORT |
Runtime | 3000 |
Frontend dev server port (dev mode only, overrides PORT) |
HOST |
Runtime | 127.0.0.1 |
Backend server host |
MCP_HOST |
Runtime | Value of HOST |
MCP server connection host (use 127.0.0.1 when HOST=0.0.0.0 on Windows) |
MCP_PORT |
Runtime | Value of BACKEND_PORT |
MCP server connection port |
DISABLE_WORKTREE_CLEANUP |
Runtime | Not set | Disable all git worktree cleanup including orphan and expired workspace cleanup (for debugging) |
VK_ALLOWED_ORIGINS |
Runtime | Not set | Comma-separated list of origins that are allowed to make backend API requests (e.g., https://my-vibekanban-frontend.com ) |
VK_SHARED_API_BASE |
Runtime | Not set | Base URL for the remote/cloud API used by the local desktop app |
VK_SHARED_RELAY_API_BASE |
Runtime | Not set | Base URL for the relay API used by tunnel-mode connections |
VK_TUNNEL |
Runtime | Not set | Enable relay tunnel mode when set (requires relay API base URL) |
Build-time variables must be set when running pnpm run build
. Runtime variables are read when the application starts.
When running Vibe Kanban behind a reverse proxy (e.g., nginx, Caddy, Traefik) or on a custom domain, you must set the VK_ALLOWED_ORIGINS
environment variable. Without this, the browser's Origin header won't match the backend's expected host, and API requests will be rejected with a 403 Forbidden error.
Set it to the full origin URL(s) where your frontend is accessible:
# Single origin
VK_ALLOWED_ORIGINS=https://vk.example.com
# Multiple origins (comma-separated)
VK_ALLOWED_ORIGINS=https://vk.example.com,https://vk-staging.example.com
When running Vibe Kanban on a remote server (e.g., via systemctl, Docker, or cloud hosting), you can configure your editor to open projects via SSH:
- Access via tunnel: Use Cloudflare Tunnel, ngrok, or similar to expose the web UI
- Configure remote SSH in Settings → Editor Integration:
- Set Remote SSH Host to your server hostname or IP
- Set Remote SSH User to your SSH username (optional)
- Prerequisites:
- SSH access from your local machine to the remote server
- SSH keys configured (passwordless authentication)
- VSCode Remote-SSH extension
When configured, the "Open in VSCode" buttons will generate URLs like vscode://vscode-remote/ssh-remote+user@host/path
that open your local editor and connect to the remote server.
See the documentation for detailed setup instructions.

---

# Pull requests · blader/humanizer - GitHub
Source URL: https://github.com/blader/humanizer/pulls

Source Type: web_page

Source ID: f6f4de6e-70e8-42b5-a5d0-f6e9048ea6d1


-
Notifications
You must be signed in to change notification settings - Fork 1.3k
Pull requests: blader/humanizer
Author
Label
Projects
Milestones
Reviews
Assignee
Sort
Pull requests list
feat: AI-iness density pre-check for adaptive pass strength (v2.6.0)
#98
opened Apr 20, 2026 by
adelaidasofia
Loading…
feat: enforce absolute ban on em dashes and en dashes
#96
opened Apr 16, 2026 by
erhanurgun
Loading…
3 tasks
feat: add quality scoring, core principles, and 3 wiki-sourced patterns
#94
opened Apr 15, 2026 by
Charpup
Loading…
4 tasks
Improve README setup instructions for Claude Code skill detection
#89
opened Apr 10, 2026 by
Gabriel-Dalton
Loading…
fix: add content preservation rule and strengthen em dash handling
#84
opened Apr 4, 2026 by
mvanhorn
Contributor
Loading…
fix: move version to metadata in SKILL.md frontmatter
#83
opened Apr 4, 2026 by
mvanhorn
Contributor
Loading…
ProTip!
Exclude everything labeled
bug
with -label:bug.

---

