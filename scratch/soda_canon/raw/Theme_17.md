# 🧠 SODA CANONICAL KNOWLEDGE BASE
> Gerado pelo @soda-knowledge-curator.
> Fontes Originais: 249 | Fontes Puras: 247 | Temas Identificados: 24



## 🧩 Eixo Temático 17

# Agent Skills in Antigravity: How to Extend Your AI Capabilities - Stephen W Thomas
Source URL: https://www.stephenwthomas.com/azure-integration-thoughts/agent-skills-antigravity-tutorial/

Source Type: web_page

Source ID: 259736b5-2b03-424c-9e14-21f21a40fc0c


The landscape of AI development is shifting from monolithic system instructions to modular, reusable capabilities. Recently, Anthropic introduced Agent Skills, an open-source standard designed to give AI agents focused, efficient tools without the overhead of complex system prompts or heavy MCP servers.
In this guide, we’ll walk through how to create your first AI agent skill directly in Antigravity, exploring why this method is the most effective way to extend your agent’s capabilities today
What are Agent Skills?
Agent skills are simple, reusable blocks of logic that follow a principle called progressive disclosure. Instead of forcing an agent to process a massive wall of system instructions at the start of every chat, it only reads the name and description of available skills.
The agent only dives deeper into the skill’s logic if it determines the skill is necessary for the task. This results in:
-
Faster Processing Times: Less initial context for the LLM to chew through.
-
Token Efficiency: You only pay for the context you actually use.
-
Cross-Platform Portability: A skill built for Antigravity can be used in Claude, Cursor, or VS Code with minimal changes.
Step-by-Step: Creating a Skill in Antigravity
Unlike other IDEs that require manual enablement, Antigravity has agent skills enabled by default. Here is how to build a simple “Game Choice” skill from scratch.
1. Define the Folder Structure
Agent skills rely on a very specific directory hierarchy. Start by creating an agent/skills/
folder in your workspace. Inside that, create a folder named after your skill (e.g., game-of-chess
).
2. Create the skill.md File
The heart of your skill is a file named skill.md
. Inside this file, you define the metadata and the logic:
Agent Skill SKILL.md for game-of-chess
---
name: game-of-chess
description: A skill to return a game choice when anyone asks to play a game.
---
# Game of Chess
When the user asks to play any game (like poker or blackjack), respond by saying: "Would you prefer a good game of chess?"
3. Advanced Folder Options
While our example is simple, you can extend skills with:
-
scripts/: Contain Python, Bash, or Node.js logic.
-
references/: Store document templates or specialized knowledge.
-
assets/: Store logos or images needed for the skill.
Testing Your New Skill
Once saved, you can immediately interact with your agent in Antigravity. By asking, “What agent skills do you have access to?”, the agent will scan your workspace and confirm it sees the game-of-chess
skill.
If you then say, “Let’s play poker,” the agent will trigger the skill and respond with the pre-defined logic: “Would you prefer a good game of chess?”
Real-World Applications
Beyond trivial game examples, agent skills are incredibly powerful for:
-
Workflow Validation: Applying corporate standards to files or code.
-
File Conversions: Handling specialized PDF or Excel manipulations.
-
Automated SEO: Analyzing video transcripts to generate chapter markers.
Don’t Start from Scratch
You don’t have to build everything yourself. The Anthropic GitHub repository already contains a library of pre-built skills for handling PDFs, PowerPoints, and document processing. You can leverage these as a starting point and simply move the folders into your local workspace.
For a deeper dive into naming standards and folder structures, visit agentskills.io for the full technical specifications.
Recent Comments

---

# Agent Skills - Google Antigravity Documentation
Source URL: https://antigravity.google/docs/skills
Source Type: web_page
Source ID: 3b87aee3-40d2-4e23-af07-c6d0143db38f

{
  "value": {
    "content": "Google Antigravity Documentation\n\nantigravity.google uses cookies from Google to deliver and enhance the quality of its services and to analyze traffic. \n\nLearn more\n\nhttps://policies.google.com/technologies/cookies?hl=en\n\nOK, got it \n\n/\n\nProduct\n\nUse Cases keyboard_arrow_down\n\nPricing\n\nBlog\n\nResources keyboard_arrow_down\n\nDownload download\n\nmenu\n\nProduct Use Cases keyboard_arrow_down\n\nBuilt for developers in the agent-first era\n\nExplore how Google Antigravity helps you build\n\nSee overview\n\n/use-cases\n\nworkspaces Professional keyboard_arrow_right\n\n/use-cases/professional\n\n \n\ncode_blocks Frontend keyboard_arrow_right\n\n/use-cases/frontend\n\n \n\nstacks Fullstack keyboard_arrow_right\n\n/use-cases/fullstack\n\nPricing Blog Resources keyboard_arrow_down\n\nEverything you need to stay up-to-date and get help\n\nDocumentation keyboard_arrow_right\n\n/docs\n\n \n\nChangelog keyboard_arrow_right\n\n/changelog\n\n \n\nSupport keyboard_arrow_right\n\n/support\n\n \n\nPress keyboard_arrow_right\n\n/press\n\n \n\nReleases keyboard_arrow_right\n\n/releases\n\nHome expand_more\n\n/docs/home\n\nGetting Started\n\n/docs/get-started\n\nAgent expand_more\n\n/docs/agent\n\nModels\n\n/docs/models\n\nAgent Modes / Settings\n\n/docs/agent-modes-settings\n\nRules / Workflows\n\n/docs/rules-workflows\n\nSkills\n\n/docs/skills\n\nTask Groups\n\n/docs/task-groups\n\nBrowser Subagent\n\n/docs/browser-subagent\n\nStrict Mode\n\n/docs/strict-mode\n\nSandboxing\n\n/docs/sandbox-mode\n\nTools expand_more\n\nMCP\n\n/docs/mcp\n\nArtifacts expand_more\n\n/docs/artifacts\n\nTask List\n\n/docs/task-list\n\nImplementation Plan\n\n/docs/implementation-plan\n\nWalkthrough\n\n/docs/walkthrough\n\nScreenshots\n\n/docs/screenshots\n\nBrowser Recordings\n\n/docs/browser-recordings\n\nKnowledge\n\n/docs/knowledge\n\nEditor expand_more\n\n/docs/editor\n\nTab\n\n/docs/tab\n\nCommand\n\n/docs/command\n\nAgent Side Panel\n\n/docs/agent-side-panel\n\nReview Changes + Source Control\n\n/docs/review-changes-editor\n\nAgent Manager expand_more\n\n/docs/agent-manager\n\nWorkspaces expand_more\n\n/docs/workspaces\n\nPlayground\n\n/docs/playground\n\nInbox\n\n/docs/inbox\n\nConversation View expand_more\n\n/docs/conversation-view\n\nBrowser Subagent View\n\n/docs/browser-subagent-view\n\nPanes\n\n/docs/panes\n\nReview Changes + Source Control\n\n/docs/review-changes-manager\n\nChanges Sidebar\n\n/docs/changes-sidebar\n\nTerminal\n\n/docs/terminal\n\nFiles\n\n/docs/files\n\nBrowser expand_more\n\n/docs/browser\n\nAllowlist / Denylist\n\n/docs/allowlist-denylist\n\nSeparate Chrome Profile\n\n/docs/separate-chrome-profile\n\nPlans\n\n/docs/plans\n\nSettings\n\n/docs/settings\n\nFAQ\n\n/docs/faq\n\nside_navigation\n\nAgent\n\n/docs/agent\n\nSkills\n\nAgent Skills\n\nSkills are an \n\nopen standard\n\nhttps://agentskills.io/home\n\n for extending agent capabilities. A skill is a folder containing a \n\nSKILL.md\n\n file with instructions that the agent can follow when working on specific tasks.\n\nWhat are skills?\n\nSkills are reusable packages of knowledge that extend what the agent can do. Each skill contains:\n\nInstructions\n\n for how to approach a specific type of task\n\nBest practices\n\n and conventions to follow\n\nOptional scripts and resources\n\n the agent can use\n\nWhen you start a conversation, the agent sees a list of available skills with their names and descriptions. If a skill looks relevant to your task, the agent reads the full instructions and follows them.\n\nWhere skills live\n\nAntigravity supports two types of skills:\n\nLocation\n\nScope\n\n<workspace-root>/.agents/skills/<skill-folder>/\n\nWorkspace-specific\n\n~/.gemini/antigravity/skills/<skill-folder>/\n\nGlobal (all workspaces)\n\nWorkspace skills\n\n are great for project-specific workflows, like your team's deployment process or testing conventions.\n\nGlobal skills\n\n work across all your projects. Use these for personal utilities or general-purpose tools you want everywhere.\n\nNote: Antigravity now defaults to .agents/skills, but still maintains backward support for .agent/skills.\n\nCreating a skill\n\nTo create a skill:\n\nCreate a folder for your skill in one of the skill directories\n\nAdd a \n\nSKILL.md\n\n file inside that folder\n\n            .agents/skills/\n└─── my-skill/\n    └─── SKILL.md\n        \n\n\nEvery skill needs a \n\nSKILL.md\n\n file with YAML frontmatter at the top:\n\n            ---\nname: my-skill\ndescription: Helps with a specific task. Use when you need to do X or Y.\n---\n\n# My Skill\n\nDetailed instructions for the agent go here.\n\n## When to use this skill\n\n- Use this when...\n- This is helpful for...\n\n## How to use it\n\nStep-by-step guidance, conventions, and patterns the agent should follow.\n        \n\n\nFrontmatter fields\n\nField\n\nRequired\n\nDescription\n\nname\n\nNo\n\nA unique identifier for the skill (lowercase, hyphens for spaces). Defaults to the folder name if not provided.\n\ndescription\n\nYes\n\nA clear description of what the skill does and when to use it. This is what the agent sees when deciding whether to apply the skill.\n\nTip: Write your description in third person and include keywords that help the agent recognize when the skill is relevant. For example: \"Generates unit tests for Python code using pytest conventions.\"\n\nSkill folder structure\n\nWhile \n\nSKILL.md\n\n is the only required file, you can include additional resources:\n\n            .agents/skills/my-skill/\n├─── SKILL.md       # Main instructions (required)\n├─── scripts/       # Helper scripts (optional)\n├─── examples/      # Reference implementations (optional)\n└─── resources/     # Templates and other assets (optional)\n        \n\n\nThe agent can read these files when following your skill's instructions.\n\nHow the agent uses skills\n\nSkills follow a \n\nprogressive disclosure\n\n pattern:\n\nDiscovery\n\n: When a conversation starts, the agent sees a list of available skills with their names and descriptions\n\nActivation\n\n: If a skill looks relevant to your task, the agent reads the full \n\nSKILL.md\n\n content\n\nExecution\n\n: The agent follows the skill's instructions while working on your task\n\nYou don't need to explicitly tell the agent to use a skill—it decides based on context. However, you can mention a skill by name if you want to ensure it's used.\n\nBest practices\n\nKeep skills focused\n\nEach skill should do one thing well. Instead of a \"do everything\" skill, create separate skills for distinct tasks.\n\nWrite clear descriptions\n\nThe description is how the agent decides whether to use your skill. Make it specific about what the skill does and when it's useful.\n\nUse scripts as black boxes\n\nIf your skill includes scripts, encourage the agent to run them with \n\n--help\n\n first rather than reading the entire source code. This keeps the agent's context focused on the task.\n\nInclude decision trees\n\nFor complex skills, add a section that helps the agent choose the right approach based on the situation.\n\nExample: A code review skill\n\nHere's a simple skill that helps the agent review code:\n\n            ---\nname: code-review\ndescription: Reviews code changes for bugs, style issues, and best practices. Use when reviewing PRs or checking code quality.\n---\n\n# Code Review Skill\n\nWhen reviewing code, follow these steps:\n\n## Review checklist\n\n1. **Correctness**: Does the code do what it's supposed to?\n2. **Edge cases**: Are error conditions handled?\n3. **Style**: Does it follow project conventions?\n4. **Performance**: Are there obvious inefficiencies?\n\n## How to provide feedback\n\n- Be specific about what needs to change\n- Explain why, not just what\n- Suggest alternatives when possible\n        \n\n\nRules / Workflows\n\n/docs/rules-workflows\n\nTask Groups\n\n/docs/task-groups\n\nOn this Page\n\nAgent Skills\n\nWhat are skills?\n\nWhere skills live\n\nCreating a skill\n\nSkill folder structure\n\nHow the agent uses skills\n\nBest practices\n\nExample: A code review skill",
    "title": "Agent Skills - Google Antigravity Documentation",
    "source_type": "web_page",
    "url": "https://antigravity.google/docs/skills",
    "char_count": 7578
  }
}


---

# udecode/skiller: Skiller — apply the same rules to all coding agents - GitHub
Source URL: https://github.com/udecode/skiller

Source Type: web_page

Source ID: 5c780d47-c2f3-4581-8aef-25fcceed996b


Apply the same rules (and skills) to multiple AI coding agents.
npx skiller@latest init
npx skiller@latest install
.agents/rules/*.mdc
is local rule authoring.agents/skills/
is the canonical runtime skill treeskills-lock.json
is the upstream source of truth for installed skillsskiller install
andskiller update
use the localskills
CLI, then auto-runapply
skiller apply
stays local and non-destructive- See docs/skills.md
- Define MCP servers once in
skiller.toml
- On
apply
, servers are propagated to all agents that support MCP - See docs/mcp.md
- docs/cli.md — commands and flags
- docs/config.md —
skiller.toml
reference - docs/skills.md — skills, propagation, plugin sync
- docs/mcp.md — MCP server config and propagation
- docs/troubleshooting.md — common failures and fixes
- docs/development.md — dev workflow
- docs/migration-from-ruler.md — notes for
ruler
users
| Identifier | Agent | Rules | MCP | Skills |
|---|---|---|---|---|
github-copilot |
GitHub Copilot | AGENTS.md |
.vscode/mcp.json (servers ) |
.agents/skills |
claude-code |
Claude Code | CLAUDE.md (@file refs) |
.mcp.json |
.claude/skills |
codex |
Codex | AGENTS.md , .codex/config.toml |
.codex/config.toml |
.agents/skills |
cursor |
Cursor | AGENTS.md |
.cursor/mcp.json |
.agents/skills |
windsurf |
Windsurf | AGENTS.md |
.windsurf/mcp_config.json |
.windsurf/skills |
cline |
Cline | .clinerules |
- | .agents/skills |
openhands |
OpenHands | .openhands/microagents/repo.md |
config.toml |
.openhands/skills |
gemini-cli |
Gemini CLI | AGENTS.md , .gemini/settings.json |
.gemini/settings.json |
.agents/skills |
junie |
Junie | .junie/guidelines.md |
- | .junie/skills |
augment |
Augment | .augment/rules/skiller_augment_instructions.md |
- | .augment/skills |
kilo |
Kilo Code | .kilocode/rules/skiller_kilocode_instructions.md |
.kilocode/mcp.json |
.kilocode/skills |
opencode |
OpenCode | AGENTS.md , opencode.json |
opencode.json |
.agents/skills |
goose |
Goose | .goosehints |
- | .goose/skills |
crush |
Crush | CRUSH.md , .crush.json |
.crush.json |
.crush/skills |
amp |
Amp | AGENTS.md |
- | .agents/skills |
qwen-code |
Qwen Code | AGENTS.md , .qwen/settings.json |
.qwen/settings.json |
.qwen/skills |
kiro-cli |
Kiro CLI | .kiro/steering/skiller_kiro_instructions.md |
- | .kiro/skills |
warp |
Warp | WARP.md |
- | .agents/skills |
roo |
Roo Code | AGENTS.md , .roo/mcp.json |
.roo/mcp.json |
.roo/skills |
trae |
Trae | .trae/rules/project_rules.md |
- | .trae/skills |

---

# The SKILL.md Pattern: How to Write AI Agent Skills That Actually Work | by Bibek Poudel
Source URL: https://bibek-poudel.medium.com/the-skill-md-pattern-how-to-write-ai-agent-skills-that-actually-work-72a3169dd7ee
Source Type: web_page
Source ID: 64cf45fb-1478-46c4-aeb2-b0d8f75a477f

{
  "value": {
    "content": "Sitemap\n\nOpen in app\n\nSign in\n\nMedium Logo\n\nWrite\n\nSearch\n\nSign in\n\nThe SKILL.md Pattern: How to Write AI Agent Skills That Actually Work\n\nBibek Poudel\n\n15 min read · Feb 26, 2026\n\n--\n\nPress enter or click to view image in full size\n\nIf your skill does not trigger, it is almost never the instructions. It is the description.\n\nThat is the thing most people figure out after an hour of frustration. You write a SKILL.md, drop it in the right folder, ask the agent to use it, and nothing happens. You rewrite the instructions. Still nothing. The problem was never what you wrote inside the skill. It was the two lines at the top that the agent uses to decide whether to activate it at all.\n\nIn this guide I want to go through exactly how Agent Skills work under the hood, why most people write that part wrong, and then build four skills together from easy to complex so you can see the pattern click into place. By the end you will have a README writer, a git commit generator, a code reviewer, and a full MCP-powered sprint planner, all built from scratch.\n\nWhat Is an Agent Skill?\n\nA Skill is not a plugin. It is not a script you wire up to an API. Think of it like writing an onboarding guide for a new team member. Instead of re-explaining your workflows and preferences in every conversation, you package them once and the agent picks them up automatically whenever your request matches.\n\nAt its core, a skill is just a folder:\n\nyour-skill-name/ \n ├── SKILL.md # Required: instructions + metadata \n ├── scripts/ # Optional: executable code the agent runs \n ├── references/ # Optional: docs loaded only when needed \n └── assets/ # Optional: templates, images, fonts\n\nThe only required file is  SKILL.md . Everything else is optional but becomes important as skills grow in complexity.\n\nWhat makes this particularly useful right now is that the SKILL.md format is an \n\nopen standard\n\n, published by Anthropic at agentskills.io in December 2025. It works across Claude Code, OpenAI Codex, and OpenClaw. While the format is standardized, each platform implements discovery and tooling slightly differently. Think shared language, not identical behavior. A skill that works on Claude Code will very likely work on Codex, but runtime behaviors like session snapshotting, tool permissions, and invocation modes differ between platforms.\n\nWhere Skills Live\n\nBefore writing anything, you need to know where to put it. Each platform loads skills from specific locations, and the location defines the scope.\n\nClaude Code:\n\nLocation Scope  ~/.claude/skills/  Personal, available across all your projects  .claude/skills/  Project-level, shared with your team via git\n\nOpenAI Codex:\n\nLocation Scope  ~/.codex/skills/  User-level, applies to any repo you work in  .codex/skills/  Repo-level, checked into git\n\nOpenClaw:\n\nLocation Scope  ~/.openclaw/skills/  Global, available to all configured agents Per-agent workspace Scoped to a specific agent only\n\nWhen two skills share the same name, the higher-precedence location wins. A project-level skill overrides a personal one with the same name. This lets teams define defaults that individuals can override for their own setups.\n\nHow the Three-Level Loading System Works\n\nThis is the part most people skip, and it explains almost every problem with skills not triggering or consuming too much context.\n\nSkills use \n\nprogressive disclosure\n\n: a three-level loading system where content is pulled into context only as it is needed.\n\nPress enter or click to view image in full size\n\nLevel 1: Metadata (always loaded, ~100 tokens per skill)\n\nAt startup, the agent reads only the  name  and  description  from every installed skill's YAML frontmatter. Nothing else. This compact listing goes into the system prompt so the agent knows what skills exist and when to use them. The practical implication: you can install many skills without a context penalty.\n\nLevel 2: Instructions (loaded when triggered, under 5k tokens)\n\nWhen the agent decides a skill is relevant, it reads the full body of  SKILL.md  into context using a bash call. Only at this point do your actual instructions get loaded.\n\nLevel 3: Referenced files and scripts (loaded on demand, effectively unlimited)\n\nIf the skill body references other files, the agent reads those only when it needs them. Scripts can be executed without being read into context at all. This is what makes skills scalable: the token cost at idle is zero regardless of how much content you bundle.\n\nHere is what this looks like in sequence for a real request:\n\n1. Session starts \n  --> Agent loads: name + description from every skill (~100 tokens each) \n 2. User asks: \"Can you write a README for this project?\" \n  --> Agent reads: readme-writer/SKILL.md full body (Level 2) \n 3. SKILL.md references a style guide file \n  --> Agent reads: readme-writer/references/style.md (Level 3) \n 4. SKILL.md includes a validation script \n  --> Agent executes: scripts/validate.sh   (runs without being read into context)\n\nSkills vs. Slash Commands\n\nBoth Claude Code and Codex support two invocation modes. The agent can activate a skill automatically when your request matches the description (implicit invocation), or you can call it directly (explicit invocation).\n\nIn \n\nClaude Code\n\n, skills appear in the slash command menu by default. You can invoke one directly with  /skill-name , or just describe what you want and Claude will activate the relevant skill automatically:\n\n# Direct invocation in Claude Code \n /readme-writer \n # Or just describe the task and Claude activates it automatically \n Can you write a README for this project?\n\nIn \n\nCodex CLI\n\n, you mention a skill with  $  prefix or use the  /skills  selector:\n\n# Explicit invocation in Codex CLI \n $readme-writer document this project \n # Or use the skill selector \n /skills\n\nEven though both platforms support explicit invocation, a well-written description still matters enormously. It is what drives automatic activation, the behavior that makes skills feel like a natural extension of how you already work rather than a command you have to remember to type.\n\nThe Most Common Mistake\n\nThe  description  field in your YAML frontmatter is not for humans. It is the trigger condition the agent uses when deciding whether to activate your skill.\n\nHere is the structure that works:\n\n[What the skill does] + [When to use it, with specific trigger phrases]\n\nBad:\n\ndescription: Helps with documents.\n\nAlso bad, because it describes what but not when:\n\ndescription: Creates sophisticated multi-page documentation with advanced  formatting.\n\nGood:\n\ndescription: Creates and writes professional README.md files for software  projects. Use when user asks to \"write a README\", \"create a readme\",  \"document this project\", \"generate project documentation\", or \"help me write  a README.md\".\n\nThe agentskills.io spec defines these constraints:\n\nname : lowercase letters, numbers, and hyphens only, max 64 characters, must not start or end with a hyphen, no consecutive hyphens\n\ndescription : max 1024 characters, must describe both what the skill does and when to use it\n\nThe file must be named exactly  SKILL.md , case-sensitive\n\nAvoid XML angle brackets ( <  or  > ) in frontmatter as they can inject unintended instructions into the system prompt\n\nSome platforms add conventions on top of these. When in doubt, check the platform-specific docs alongside the base spec at agentskills.io/specification.\n\nSkill 1: README Writer\n\nThis is a practical first skill. Almost every developer has written a README at some point, usually by hand, usually inconsistently. This skill teaches the agent your preferred structure and writes the file to disk automatically.\n\nSetup\n\nmkdir -p ~/.claude/skills/readme-writer\n\nSKILL.md\n\n--- \n name: readme-writer \n description: Creates and writes professional README.md files for software projects. \n Use when user asks to \"write a README\", \"create a readme\", \"document this project\", \n \"generate project documentation\", or \"help me write a README.md\". Works from a \n project description, existing code, or both. \n --- \n # README Writer \n ## Overview \n Generate a complete, professional README.md file and write it to disk. The output \n should be clear enough for a first-time contributor to understand the project, \n set it up locally, and start contributing. \n ## Step 1: Gather project context \n Look for context in the codebase before asking the user: \n ```bash \n ls -la \n cat package.json 2>/dev/null || cat pyproject.toml 2>/dev/null || \\ \n  cat go.mod 2>/dev/null || echo \"No manifest found\" \n ls .env.example .env.sample 2>/dev/null || echo \"No env example found\" \n ``` \n Gather: \n - What does this project do? (1-2 sentence summary) \n - What language and main frameworks does it use? \n - How do you install and run it? \n - Are there environment variables needed? \n - Is there a LICENSE file? \n ## Step 2: Write the README \n Use this structure. Only include sections that are relevant. Do not add empty sections. \n ``` \n # Project Name \n One clear sentence describing what this project does and who it is for. \n ## Features \n - Feature one (be specific) \n - Feature two \n ## Prerequisites \n List what needs to be installed. Include version requirements if important. \n ## Installation \n Step-by-step setup. Every command must be copy-pasteable. \n ```bash \n git clone https://github.com/username/project \n cd project \n npm install \n ``` \n ## Configuration \n If the project needs environment variables, show an example: \n ```bash \n cp .env.example .env \n ``` \n Then explain each variable the user needs to set manually. \n ## Usage \n Show the most common use case first. \n ```bash \n npm run dev \n ``` \n ## License \n [MIT](LICENSE) \n ``` \n ## Step 3: Write the file to disk \n Once the content is ready, write it: \n ```bash \n cat > README.md << 'EOF' \n [full readme content] \n EOF \n ``` \n Confirm it was written: \n ```bash \n echo \"README.md written: $(wc -l < README.md) lines\" \n ``` \n ## Step 4: Quality check \n Before finishing, verify: \n - [ ] No placeholder text like \"[your description here]\" remains \n - [ ] Every command in the Installation section is accurate for this project \n - [ ] Prerequisites match what the project actually needs \n - [ ] License section matches the LICENSE file if one exists \n ```\n\nTest it\n\nGo to any project folder and ask:\n\nCan you write a README for this project?\n\nThe agent will inspect the codebase, write the README, save it as `README.md`, and confirm with a line count. No copy-pasting required.\n\nSkill 2: Git Commit Message Generator\n\nThis skill shows how to write trigger phrases that cover the different ways a developer might ask for the same thing.\n\nSetup\n\nmkdir -p ~/.claude/skills/git-commit-writer\n\nSKILL.md\n\n--- \n name: git-commit-writer \n description: Generates standardized git commit messages following conventional commits spec. \n Use when user asks to \"write a commit message\", \"help me commit\", \"summarize my changes\", \n \"what should my commit say\", or \"draft a commit\". Analyzes staged diffs and change \n descriptions to produce type(scope): description format messages. \n --- \n # Git Commit Message Writer \n ## Format \n ``` \n type(scope): short description \n [optional body] \n [optional footer] \n ``` \n Allowed types: feat, fix, docs, style, refactor, test, chore, perf, ci, build \n ## Instructions \n ### Step 1: Get the diff \n ```bash \n git diff --staged \n ``` \n If nothing is staged: \n ```bash \n git diff HEAD \n ``` \n ### Step 2: Analyze the changes \n Look for: \n - What files changed and what category they belong to \n - Whether this adds new functionality (feat), fixes a bug (fix), or updates docs/config/tests \n - The scope: which module, component, or area is affected \n ### Step 3: Write the message \n - Keep the subject line under 72 characters \n - Use imperative mood: \"add feature\" not \"added feature\" \n - Do not end the subject line with a period \n - Add a body if the change needs more context than the subject allows \n ### Quality check \n - [ ] Type is one of the allowed types \n - [ ] Subject line is under 72 characters \n - [ ] Imperative mood is used \n - [ ] Scope is specific enough to be useful \n ## Examples \n ``` \n feat(auth): add OAuth2 login with Google \n Implements Google OAuth2 flow using the existing session management \n system. Users can now sign in with their Google account. \n Closes #142 \n ``` \n ``` \n fix(api): handle null response from payment provider \n ``` \n ``` \n docs(readme): update local setup instructions for Node 22 \n ``` \n ```\n\nSkill 3: Code Reviewer (Multi-File)\n\nThis skill shows when to split content across multiple files. The process stays in `SKILL.md`. The detailed criteria live in a reference file loaded only during an actual review. This is the right way to structure complex skills.\n\nmkdir -p ~/.claude/skills/code-reviewer/references\n\nSKILL.md\n\n--- \n name: code-reviewer \n description: Conducts structured code reviews with categorized feedback. Use when user asks \n to \"review this code\", \"check my PR\", \"look over this function\", or \"give me feedback on \n this implementation\". Produces structured output with blocking issues separate from suggestions. \n --- \n # Code Reviewer \n ## Review Process \n ### Step 1: Understand context \n Before reviewing, establish: \n - What is this code supposed to do? \n - What language and framework is it using? \n - Is this a new feature, a bug fix, or a refactor? \n ### Step 2: Run the review \n For detailed review criteria by category, see [references/criteria.md](references/criteria.md). \n Work through each category in order. Do not skip categories even if they seem unlikely to have issues. \n ### Step 3: Structure the output \n ``` \n ## Summary \n [2-3 sentence overview and overall assessment] \n ## Blocking Issues \n [Issues that must be fixed: security vulnerabilities, logic errors, data loss risks. \n If none, write \"None found.\"] \n ## Suggestions \n [Non-blocking improvements numbered. Include where, why, and how to fix each.] \n ## Positive Notes \n [What the code does well. Always include at least one.] \n ``` \n ``` \n\n\nreferences/criteria.md\n\n# Review Criteria \n ## Security (Check First) \n - SQL injection: are user inputs parameterized? \n - XSS: is output properly escaped before rendering? \n - Auth checks: are protected routes actually protected? \n - Secrets: are API keys or credentials hardcoded anywhere? \n - Input validation: is validation happening server-side? \n ## Correctness \n - Does the logic match the stated intent? \n - Are edge cases handled: empty arrays, null values, zero, negative numbers? \n - Are error states surfaced correctly? \n - Are async operations awaited properly? \n ## Readability \n - Can a new team member understand this in 5 minutes? \n - Are variable and function names descriptive? \n - Are functions doing one thing or multiple things? \n ## Performance \n - Are there obvious N+1 query patterns? \n - Are expensive operations inside loops that could be outside? \n ## Tests \n - Are there tests for the new behavior? \n - Are edge cases tested, not just the happy path?\n\nThe  SKILL.md  body stays under 40 lines. The detailed criteria live in  references/criteria.md  and are loaded only when a review is running. This keeps Level 2 lean while the agent still has access to everything it needs at Level 3.\n\nSkill 4: Linear Sprint Planner with MCP\n\nThis is a Category 3 skill: MCP Enhancement. The MCP server gives the agent access to Linear’s API. The skill gives it the knowledge of how to use that access reliably and consistently. Without the skill, users connect the MCP but still have to figure out every step. With the skill, the entire workflow runs from one sentence.\n\nSetup\n\nmkdir -p ~/.claude/skills/linear-sprint-planner/references\n\nSKILL.md\n\n--- \n name: linear-sprint-planner \n description: Automates Linear sprint planning including cycle creation, backlog triage, \n and task assignment. Use when user says \"plan the sprint\", \"set up the next cycle\", \n \"help me prioritize the backlog\", or \"create sprint tasks in Linear\". Requires Linear \n MCP server to be connected. \n metadata: \n  mcp-server: linear \n  version: 1.0.0 \n --- \n # Linear Sprint Planner \n ## Prerequisites \n Verify the Linear MCP server is connected. If not available, tell the user to connect \n it in their MCP settings before continuing. \n ## Process \n ### Step 1: Gather current state \n Fetch from Linear in sequence: \n 1. Current active cycle and completion percentage \n 2. All backlog issues (status: Backlog or Todo, not assigned to any cycle) \n 3. Team members and current workload \n 4. Any issues marked high priority \n See [references/linear-api.md](references/linear-api.md) for pagination and rate limit handling. \n ### Step 2: Analyze capacity \n - Count team members participating in the sprint \n - Estimate points (default: 10 per person per week unless historical velocity exists) \n - Subtract planned time off \n Present a summary before proceeding: \n Team capacity: - [N] engineers x [X] points = [Total] available - Carrying over: [X] points - Net new capacity: [X] points \n ### Step 3: Prioritize backlog \n Sort in this order: \n 1. P0/P1 bugs and blockers (always include) \n 2. Items explicitly flagged by the user \n 3. Items that unblock other teams \n 4. Features by product priority \n 5. Tech debt \n Do not exceed capacity by more than 10% unless the user asks. \n ### Step 4: Present for approval \n Before creating anything in Linear, show the proposed plan: \n Proposed Sprint [N]: [Date Range] Capacity: [X] points Issues: - [ISSUE-123] Fix payment timeout (P0) - 3 pts - [ISSUE-456] Add CSV export (P2) - 5 pts - [ISSUE-789] Refactor auth middleware - 2 pts Total: [X] / [X] points Shall I create this cycle and assign these issues? \n Always wait for confirmation before making changes. \n ### Step 5: Create and confirm \n Once approved: \n 1. Create the cycle with the agreed date range \n 2. Add each issue to the cycle \n 3. Update assignments if the user specified owners \n 4. Return a summary with the Linear cycle link \n See [references/error-handling.md](references/error-handling.md) if any API calls fail.\n\nreferences/linear-api.md\n\n# Linear API Patterns \n ## Fetching backlog \n Paginate in batches of 50. Check pageInfo.hasNextPage and use the after cursor. \n Filter for: status [Backlog, Todo], cycle: null. \n ## Creating a cycle \n Required: name, startsAt, endsAt, teamId \n ## Adding issues \n Use issueUpdate mutation to set cycleId. Batch where possible. \n ## Rate limiting \n On 429: wait 1 second, retry once. If it fails again, report to user and continue.\n\nreferences/error-handling.md\n\n# Error Handling \n ## MCP connection errors \n Tell the user: \"Linear MCP appears to be disconnected. Please reconnect before \n running sprint planning.\" Do not proceed. \n ## Missing data \n Continue with available data. Note what is missing in the final summary. \n ## Cycle creation failure \n Do not attempt to add issues. Report the error and suggest checking permissions.\n\nTest it\n\nHelp me plan the next sprint in Linear\n\nThe agent fetches live data, proposes a plan, waits for approval, and executes. One sentence triggers the entire workflow.\n\nOne important thing to internalize before moving on: skills do not guarantee execution. The model still decides whether to follow the instructions. Think of them as structured guidance that dramatically increases consistency, not deterministic automation. If the model goes off-script, the fix is almost always improving the instructions or the description, not debugging runtime behavior.\n\nHow All Four Skills Relate\n\nHere is a visual of the four skills and where they sit on the complexity spectrum:\n\nPress enter or click to view image in full size\n\nStart with  git-commit-writer . When you need file output, use the  readme-writer  pattern. When your skill gets too long, split it like  code-reviewer . When you need external tool coordination, use the  linear-sprint-planner  pattern.\n\nThe  allowed-tools  Field\n\nOne frontmatter field most people never use is  allowed-tools . It restricts which tools the agent can call when a skill is active.\n\nThis is useful for read-only skills where you do not want the agent accidentally writing or executing anything:\n\n--- \n name: log-analyzer \n description: Analyzes application log files to identify errors and patterns. Use when \n user says \"check the logs\", \"what errors are in my logs\", or \"analyze this log file\". \n allowed-tools: Read, Grep, Glob \n --- \n # Log Analyzer \n 1. Use Glob to find log files: *.log, logs/*.log, /var/log/*.log \n 2. Use Grep to search for error patterns: ERROR, FATAL, Exception, Traceback \n 3. Group errors by type and frequency \n 4. Summarize with the most frequent errors first\n\nWith this active, the agent cannot execute shell commands, write files, or make any external calls. A simple way to add safety guarantees to observational skills.\n\nNote:  allowed-tools  is marked experimental in the Agent Skills spec, and support varies between agent implementations. It is well-supported in Claude Code today.\n\nSharing Skills with Your Team\n\nThe cleanest approach is to commit project skills to your repo:\n\nmkdir -p .claude/skills/readme-writer \n # add SKILL.md, then: \n git add .claude/skills/ \n git commit -m \"Add readme-writer skill for team\" \n git push\n\nWhen teammates pull the repo, the skill is immediately available with no separate installation step. In Codex the equivalent path is  .codex/skills/ .\n\nDebugging When a Skill Does Not Trigger\n\n1. Check your description\n\nThe most common issue. Add more specific trigger phrases that match how users actually phrase requests.\n\n2. Check your file path\n\n# Claude Code personal skill \n ls ~/.claude/skills/your-skill/SKILL.md \n # Claude Code project skill \n ls .claude/skills/your-skill/SKILL.md \n # Codex \n ls ~/.codex/skills/your-skill/SKILL.md\n\n3. Check YAML syntax\n\nInvalid YAML silently prevents loading. Frontmatter must start on line 1 with  ---  and close with another  --- .\n\n4. Restart the session\n\nSkills are snapshotted at session start. Edits made during a running session require a restart to take effect.\n\n5. Run in debug mode\n\nclaude --debug\n\n6. Test with explicit invocation\n\nUse the readme-writer skill to document this project\n\nIf it works when invoked explicitly but not automatically, the description needs more specific trigger phrases.\n\nSecurity\n\nSkills can bundle executable code and instructions that control agent behavior. That same power makes malicious skills dangerous.\n\nInstall skills only from trusted sources. Before installing any community skill, read every file in the folder, especially anything in  scripts/ . Pay attention to instructions that tell the agent to make outbound network calls or send data to external services.\n\nCisco researchers have warned about skills being used for silent data exfiltration via prompt injection. Security audits scanning thousands of community skills have found a meaningful fraction with critical vulnerabilities including credential theft and malware. Snyk has published findings on this specifically. ClawHub has a VirusTotal integration you can use to check skills before installing, but manual review is still worthwhile for anything with broad permissions.\n\nPractical rules:\n\nNever install a skill that asks you to paste secrets into chat\n\nAlways read  scripts/  before installing\n\nBe skeptical of skills that make outbound network calls in setup instructions\n\nPrefer skills from official sources: Anthropic’s or OpenAI’s skills repos, or your own team’s\n\nWrapping Up\n\nThe three-level loading system is the core concept. Everything else follows from it. Level 1 is your trigger. Level 2 is your runbook. Level 3 is your reference library. Get those right and you can build anything from a single Markdown file to a multi-step workflow coordinating external APIs and executing code.\n\nThe SKILL.md format is also no longer just a Claude feature. OpenAI adopted it for Codex. OpenClaw uses it as its core plugin format. Skills you write today are portable across all three platforms. That portability is worth investing in.\n\nFeel free to modify and expand the skills built in this guide to suit your specific workflows. The official skills repos are good places to study well-written examples from both Anthropic and OpenAI:\n\nAnthropic skills: \n\nhttps://github.com/anthropics/skills\n\nhttps://github.com/anthropics/skills\n\nOpenAI skills: \n\nhttps://github.com/openai/skills\n\nhttps://github.com/openai/skills\n\nReferences:\n\nAnthropic Agent Skills Overview: \n\nhttps://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview\n\nClaude Code Skills Guide: \n\nhttps://code.claude.com/docs/en/skills\n\nOpenAI Codex Skills: \n\nhttps://developers.openai.com/codex/skills\n\nAgent Skills Open Standard: \n\nhttps://agentskills.io/what-are-skills\n\nThe Complete Guide to Building Skills for Claude: \n\nhttps://resources.anthropic.com/hubfs/The-Complete-Guide-to-Building-Skill-for-Claude.pdf\n\nEquipping Agents for the Real World: \n\nhttps://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills\n\nAgents\n\nhttps://github.com/openai/skills\n\nArtificial Intelligence\n\nhttps://github.com/openai/skills\n\nSoftware Engineering\n\nhttps://github.com/openai/skills\n\nLLM\n\nhttps://github.com/openai/skills\n\nProgramming\n\nhttps://github.com/openai/skills\n\nWritten by  Bibek Poudel\n\n140 followers\n\nhttps://github.com/openai/skills\n\n·\n\n13 following\n\nhttps://bibek-poudel.medium.com/following?source=post_page---post_author_info--72a3169dd7ee---------------------------------------\n\nAI/ML Engineer | Building, learning, and sharing ideas\n\nNo responses yet\n\nHelp\n\nStatus\n\nAbout\n\nCareers\n\nPress\n\nBlog\n\nPrivacy\n\nRules\n\nTerms\n\nText to speech",
    "title": "The SKILL.md Pattern: How to Write AI Agent Skills That Actually Work | by Bibek Poudel",
    "source_type": "web_page",
    "url": "https://bibek-poudel.medium.com/the-skill-md-pattern-how-to-write-ai-agent-skills-that-actually-work-72a3169dd7ee",
    "char_count": 25630
  }
}


---

# Authoring Google Antigravity Skills
Source URL: https://codelabs.developers.google.com/getting-started-with-antigravity-skills

Source Type: web_page

Source ID: 7a9cc93c-f887-434f-bf5e-952310d225a9


1. Introduction
Google Antigravity is an agentic IDE from Google. In this codelab, we will use Antigravity to build Agent Skills, a lightweight, open format for extending AI agent capabilities with specialized knowledge and workflows. You will be able to learn what Agent Skills are, their benefits and how they are constructed. You will then build multiple Agent Skills ranging from a Git formatter, template generator, tool code scaffolding and more, all usable within Antigravity.
Prerequisites:
- Google Antigravity installed and configured.
- Basic understanding of Google Antigravity. It is recommended to complete the codelab: Getting Started with Google Antigravity.
2. Why Skills
Modern AI agents have evolved from simple listeners to complex reasoners that integrate with local file systems and external tools (via MCP servers). However, indiscriminately loading an agent with entire codebases and hundreds of tools leads to Context Saturation and "Tool Bloat." Even with large context windows, dumping 40–50k tokens of unused tools into active memory causes high latency, financial waste, and "context rot," where the model becomes confused by irrelevant data.
The Solution: Agent Skills
To solve this, Anthropic introduced Agent Skills, shifting the architecture from monolithic context loading to Progressive Disclosure. Instead of forcing the model to "memorize" every specific workflow (like database migrations or security audits) at the start of a session, these capabilities are packaged into modular, discoverable units.
How It Works
The model is initially exposed only to a lightweight "menu" of metadata. It loads the heavy procedural knowledge (instructions and scripts) only when the user's intent specifically matches a skill. This ensures that a developer asking to refactor authentication middleware gets security context without loading unrelated CSS pipelines, keeping the context lean, fast, and cost-effective.
3. Agent Skills and Antigravity
In the Antigravity ecosystem, if the Agent Manager is the brain and the Editor is the canvas, Skills act as specialized training modules that bridge the gap between the generalist Gemini 3 model and your specific context. They allow the agent to "equip" a defined set of instructions and protocols—such as database migration standards or security checks—only when a relevant task is requested. By dynamically loading these execution protocols, Skills effectively transform the AI from a generic programmer into a specialist that rigorously adheres to an organization's codified best practices and safety standards.
What is a Skill in Antigravity?
In the context of Google Antigravity, a Skill is a directory-based package containing a definition file (SKILL.md
) and optional supporting assets (scripts, references, templates).
It is a mechanism for on-demand capability extension.
- On-Demand: Unlike a System Prompt (which is always loaded), a Skill is only loaded into the agent's context when the agent determines it is relevant to the user's current request. This optimizes the context window and prevents the agent from being distracted by irrelevant instructions. In large projects with dozens of tools, this selective loading is crucial for performance and reasoning accuracy.
- Capability Extension: Skills can do more than just instruct; they can execute. By bundling Python or Bash scripts, a Skill can give the agent the ability to perform complex, multi-step actions on the local machine or external networks without the user needing to manually run commands. This transforms the agent from a text generator into a tool user.
Skills v/s the Ecosystem (Tools, Rules and Workflows)
While Model Context Protocol (MCP) functions as the agent's "hands"—providing heavy-duty, persistent connections to external systems like GitHub or PostgreSQL—Skills act as the "brains" that direct them.
MCP handles the stateful infrastructure, whereas Skills are lightweight, ephemeral task definitions that package the methodology for using those tools. This serverless approach allows agents to execute ad-hoc tasks (like generating changelogs or migrations) without the operational overhead of running persistent processes, loading the context only when the task is active and releasing it immediately after.
Functionally, Skills occupy a unique middle ground between "Rules" (passive, always-on guardrails) and "Workflows" (active, user-triggered macros). Unlike workflows that require specific commands (e.g., /test
), Skills are agent-triggered: the model automatically detects the user's intent and dynamically equips the specific expertise required. This architecture allows for powerful composability; for example, a global Rule can enforce the use of a "Safe-Migration" Skill during database changes, or a single Workflow can orchestrate multiple Skills to build a robust deployment pipeline.
4. Creating Skills
Creating a Skill in Antigravity follows a specific directory structure and file format. This standardization ensures that skills are portable and that the agent can reliably parse and execute them. The design is intentionally simple, relying on widely understood formats like Markdown and YAML, lowering the barrier to entry for developers wishing to extend their IDE's capabilities.
Directory Structure
Skills can be defined at two scopes, allowing for both project-specific and user-specific customizations :
- Workspace Scope: Located in
<workspace-root>/.agent/skills/
. These skills are available only within the specific project. This is ideal for project-specific scripts, such as deployment to a specific environment, database management for that app, or generating boilerplate code for a proprietary framework. - Global Scope: Located in
~/.gemini/antigravity/skills/
. These skills are available across all projects on the user's machine. This is suitable for general utilities like "Format JSON," "Generate UUIDs," "Review Code Style," or integration with personal productivity tools.
A typical Skill directory looks like this:
my-skill/
├── SKILL.md # The definition file
├── scripts/ # [Optional] Python, Bash, or Node scripts
├── run.py
└── util.sh
├── references/ # [Optional] Documentation or templates
└── api-docs.md
└── assets/ # [Optional] Static assets (images, logos)
This structure separates concerns effectively. The logic (scripts
) is separated from the instruction (SKILL.md
) and the knowledge (references
), mirroring standard software engineering practices.
The SKILL.md Definition File
The SKILL.md file is the brain of the Skill. It tells the agent what the skill is, when to use it, and how to execute it.
It consists of two parts:
- YAML Frontmatter
- Markdown Body.
YAML Frontmatter
This is the metadata layer. It is the only part of the skill that is indexed by the agent's high-level router. When a user sends a prompt, the agent semantic-matches the prompt against the description fields of all available skills.
---
name: database-inspector
description: Use this skill when the user asks to query the database, check table schemas, or inspect user data in the local PostgreSQL instance.
---
Key Fields:
- name: This is not mandatory. Must be unique within the scope. Lowercase, hyphens allowed (e.g.,
postgres-query
,pr-reviewer
). If it's not provided, it will default to the directory name. - description: This is mandatory and the most important field. It functions as the "trigger phrase." It must be descriptive enough for the LLM to recognize semantic relevance. A vague description like "Database tools" is insufficient. A precise description like "Executes read-only SQL queries against the local PostgreSQL database to retrieve user or transaction data. Use this for debugging data states" ensures the skill is picked up correctly.
The Markdown Body
The body contains the instructions. This is "prompt engineering" persisted to a file. When the skill is activated, this content is injected into the agent's context window.
The body should include:
- Goal: A clear statement of what the skill achieves.
- Instructions: Step-by-step logic.
- Examples: Few-shot examples of inputs and outputs to guide the model's performance.
- Constraints: "Do not" rules (e.g., "Do not run DELETE queries").
Example SKILL.md Body:
Database Inspector
Goal
To safely query the local database and provide insights on the current data state.
Instructions
- Analyze the user's natural language request to understand the data need.
- Formulate a valid SQL query.
- CRITICAL: Only SELECT statements are allowed.
- Use the script scripts/query_runner.py to execute the SQL.
- Command: python scripts/query_runner.py "SELECT * FROM..."
- Present the results in a Markdown table.
Constraints
- Never output raw user passwords or API keys.
- If the query returns > 50 rows, summarize the data instead of listing it all.
Script Integration
One of the most powerful features of Skills is the ability to delegate execution to scripts. This allows the agent to perform actions that are difficult or impossible for an LLM to do directly (like binary execution, complex mathematical calculation, or interacting with legacy systems).
Scripts are placed in the scripts/
subdirectory. The SKILL.md
references them by relative path.
5. Authoring Skills
The goal of this section is to build out Skills that integrate into Antigravity and progressively show various features like resources / scripts / etc.
You can download the Skills from the Github repo here: https://github.com/rominirani/antigravity-skills.
We can look at placing each of these skills inside of either ~/.gemini/antigravity/skills
folder or /.agent/skills
folder.
Level 1 : The Basic Router ( git-commit-formatter
)
Let's consider this as the "Hello World" of Skills.
Developers often write lazy commit messages e.g. "wip", "fix bug", "updates". Enforcing "Conventional Commits" manually is tedious and often forgotten. Let's implement a Skill that enforces the Conventional Commits specification. By simply instructing the agent on the rules, we allow it to act as the enforcer.
git-commit-formatter/
└── SKILL.md (Instructions only)
The SKILL.md
file is shown below:
---
name: git-commit-formatter
description: Formats git commit messages according to Conventional Commits specification. Use this when the user asks to commit changes or write a commit message.
---
Git Commit Formatter Skill
When writing a git commit message, you MUST follow the Conventional Commits specification.
Format
`<type>[optional scope]: <description>`
Allowed Types
- **feat**: A new feature
- **fix**: A bug fix
- **docs**: Documentation only changes
- **style**: Changes that do not affect the meaning of the code (white-space, formatting, etc)
- **refactor**: A code change that neither fixes a bug nor adds a feature
- **perf**: A code change that improves performance
- **test**: Adding missing tests or correcting existing tests
- **chore**: Changes to the build process or auxiliary tools and libraries such as documentation generation
Instructions
1. Analyze the changes to determine the primary `type`.
2. Identify the `scope` if applicable (e.g., specific component or file).
3. Write a concise `description` in an imperative mood (e.g., "add feature" not "added feature").
4. If there are breaking changes, add a footer starting with `BREAKING CHANGE:`.
Example
`feat(auth): implement login with google`
How to Run This Example:
- Make a small change to any file in your workspace.
- Open the chat and type: Commit these changes.
- The Agent will not just run git commit. It will first activate the git-commit-formatter skill.
- Result: A conventional Git commit message will be proposed.
For e.g. I made Antigravity add some comments to a sample Python file and it ended up with a Git commit message like docs: add detailed comments to demo_primes.py.
Level 2: Asset Utilization (license-header-adder)
This is the "Reference" pattern.
Every source file in a corporate project might need a specific 20-line Apache 2.0 license header. Putting this static text directly into the prompt (or SKILL.md
) is wasteful. It consumes tokens every time the skill is indexed, and the model might "hallucinate" typos in legal text.
Offloading the static text to a plain text file in a resources/
folder. The skill instructs the agent to read this file only when needed.
license-header-adder/
├── SKILL.md
└── resources/
└── HEADER_TEMPLATE.txt (The heavy text)
The SKILL.md
file is shown below:
---
name: license-header-adder
description: Adds the standard open-source license header to new source files. Use involves creating new code files that require copyright attribution.
---
# License Header Adder Skill
This skill ensures that all new source files have the correct copyright header.
## Instructions
1. **Read the Template**:
First, read the content of the header template file located at `resources/HEADER_TEMPLATE.txt`.
2. **Prepend to File**:
When creating a new file (e.g., `.py`, `.java`, `.js`, `.ts`, `.go`), prepend the `target_file` content with the template content.
3. **Modify Comment Syntax**:
- For C-style languages (Java, JS, TS, C++), keep the `/* ... */` block as is.
- For Python, Shell, or YAML, convert the block to use `#` comments.
- For HTML/XML, use `<!-- ... -->`.
How to Run This Example:
- Create a new dummy python file:
touch my_script.py
- Type:
Add the license header to my_script.py
. - The agent will read
license-header-adder/resources/HEADER_TEMPLATE.txt
. - It will paste the content exactly, verbatim, into your file.
Level 3: Learning by Example (json-to-pydantic)
The "Few-Shot" pattern.
Converting loose data (like a JSON API response) to strict code (like Pydantic models) involves dozens of decisions. How should we name the classes? Should we use Optional
? snake_case
or camelCase
? Writing out these 50 rules in English is tedious and error-prone.
LLMs are pattern-matching engines.
Showing them a golden example (Input
-> Output
) is often more effective than verbose instructions.
json-to-pydantic/
├── SKILL.md
└── examples/
├── input_data.json (The Before State)
└── output_model.py (The After State)
The SKILL.md
file is shown below:
---
name: json-to-pydantic
description: Converts JSON data snippets into Python Pydantic data models.
---
# JSON to Pydantic Skill
This skill helps convert raw JSON data or API responses into structured, strongly-typed Python classes using Pydantic.
Instructions
1. **Analyze the Input**: Look at the JSON object provided by the user.
2. **Infer Types**:
- `string` -> `str`
- `number` -> `int` or `float`
- `boolean` -> `bool`
- `array` -> `List[Type]`
- `null` -> `Optional[Type]`
- Nested Objects -> Create a separate sub-class.
3. **Follow the Example**:
Review `examples/` to see how to structure the output code. notice how nested dictionaries like `preferences` are extracted into their own class.
- Input: `examples/input_data.json`
- Output: `examples/output_model.py`
Style Guidelines
- Use `PascalCase` for class names.
- Use type hints (`List`, `Optional`) from `typing` module.
- If a field can be missing or null, default it to `None`.
In the /examples
folder , there is the JSON file and the output file i.e. Python file. Both of them are shown below:
input_data.json
{
"user_id": 12345,
"username": "jdoe_88",
"is_active": true,
"preferences": {
"theme": "dark",
"notifications": [
"email",
"push"
]
},
"last_login": "2024-03-15T10:30:00Z",
"meta_tags": null
}
output_model.py
from pydantic import BaseModel, Field
from typing import List, Optional
class Preferences(BaseModel):
theme: str
notifications: List[str]
class User(BaseModel):
user_id: int
username: str
is_active: bool
preferences: Preferences
last_login: Optional[str] = None
meta_tags: Optional[List[str]] = None
How to Run This Example:
- Provide the agent with a snippet of JSON (paste it in chat or point to a file).
{ "product": "Widget", "cost": 10.99, "stock": null }
- Type:
Convert this JSON to a Pydantic model
. - The agent looks at the
examples
pair in the skill folder. - It generates a Python class that perfectly mimics the coding style, imports, and structure of
output_model.py
, including handling the null stock as Optional.
A sample output (product_model.py
) is shown below:
from pydantic import BaseModel
from typing import Optional
class Product(BaseModel):
product: str
cost: float
stock: Optional[int] = None
Level 4: Procedural Logic (database-schema-validator)
This is the "Tool Use" Pattern.
If you ask an LLM "Is this schema safe?", it might say all is well, even if a critical primary key is missing, simply because the SQL looks correct.
Let's delegate this check to a deterministic Script. We use the Skill to route the agent to run a Python script that we wrote. The script provides binary (True/False) truth.
database-schema-validator/
├── SKILL.md
└── scripts/
└── validate_schema.py (The Validator)
The SKILL.md
file is shown below:
---
name: database-schema-validator
description: Validates SQL schema files for compliance with internal safety and naming policies.
---
# Database Schema Validator Skill
This skill ensures that all SQL files provided by the user comply with our strict database standards.
Policies Enforced
1. **Safety**: No `DROP TABLE` statements.
2. **Naming**: All tables must use `snake_case`.
3. **Structure**: Every table must have an `id` column as PRIMARY KEY.
Instructions
1. **Do not read the file manually** to check for errors. The rules are complex and easily missed by eye.
2. **Run the Validation Script**:
Use the `run_command` tool to execute the python script provided in the `scripts/` folder against the user's file.
`python scripts/validate_schema.py <path_to_user_file>`
3. **Interpret Output**:
- If the script returns **exit code 0**: Tell the user the schema looks good.
- If the script returns **exit code 1**: Report the specific error messages printed by the script to the user and suggest fixes.
The validate_schema.py
file is shown below:
import sys
import re
def validate_schema(filename):
"""
Validates a SQL schema file against internal policy:
1. Table names must be snake_case.
2. Every table must have a primary key named 'id'.
3. No 'DROP TABLE' statements allowed (safety).
"""
try:
with open(filename, 'r') as f:
content = f.read()
lines = content.split('\n')
errors = []
# Check 1: No DROP TABLE
if re.search(r'DROP TABLE', content, re.IGNORECASE):
errors.append("ERROR: 'DROP TABLE' statements are forbidden.")
# Check 2 & 3: CREATE TABLE checks
table_defs = re.finditer(r'CREATE TABLE\s+(?P<name>\w+)\s*\((?P<body>.*?)\);', content, re.DOTALL | re.IGNORECASE)
for match in table_defs:
table_name = match.group('name')
body = match.group('body')
# Snake case check
if not re.match(r'^[a-z][a-z0-9_]*$', table_name):
errors.append(f"ERROR: Table '{table_name}' must be snake_case.")
# Primary key check
if not re.search(r'\bid\b.*PRIMARY KEY', body, re.IGNORECASE):
errors.append(f"ERROR: Table '{table_name}' is missing a primary key named 'id'.")
if errors:
for err in errors:
print(err)
sys.exit(1)
else:
print("Schema validation passed.")
sys.exit(0)
except FileNotFoundError:
print(f"Error: File '{filename}' not found.")
sys.exit(1)
if __name__ == "__main__":
if len(sys.argv) != 2:
print("Usage: python validate_schema.py <schema_file>")
sys.exit(1)
validate_schema(sys.argv[1])
How to Run This Example:
- Create a bad SQL file
bad_schema.sql
:CREATE TABLE users (name TEXT);
- Type:
Validate bad_schema.sql
. - The agent does not guess. It will invoke the script, which fails (Exit Code 1) and it will report to us that "The validation failed because the table ‘users' is missing a primary key."
Level 5: The Architect (adk-tool-scaffold)
This pattern covers most of the features available in Skills.
Complex tasks often require a sequence of operations that combine everything we've seen: creating files, following templates, and writing logic. Creating a new Tool for the ADK (Agent Development Kit) requires all of this.
We combine:
- Script (to handle the file creation/scaffolding)
- Template (to handle boilerplate in resources)
- An Example (to guide the logic generation).
adk-tool-scaffold/
├── SKILL.md
├── resources/
│ └── ToolTemplate.py.hbs (Jinja2 Template)
├── scripts/
│ └── scaffold_tool.py (Generator Script)
└── examples/
└── WeatherTool.py (Reference Implementation)
The SKILL.md
file is shown below. You can refer to the repository of skills to check the files in the scripts, resources and examples folder. For this specific Skill, go to the adk-tool-scaffold
skill.
---
name: adk-tool-scaffold
description: Scaffolds a new custom Tool class for the Agent Development Kit (ADK).
---
# ADK Tool Scaffold Skill
This skill automates the creation of standard `BaseTool` implementations for the Agent Development Kit.
Instructions
1. **Identify the Tool Name**:
Extract the name of the tool the user wants to build (e.g., "StockPrice", "EmailSender").
2. **Review the Example**:
Check `examples/WeatherTool.py` to understand the expected structure of an ADK tool (imports, inheritance, schema).
3. **Run the Scaffolder**:
Execute the python script to generate the initial file.
`python scripts/scaffold_tool.py <ToolName>`
4. **Refine**:
After generation, you must edit the file to:
- Update the `execute` method with real logic.
- Define the JSON schema in `get_schema`.
Example Usage
User: "Create a tool to search Wikipedia."
Agent:
1. Runs `python scripts/scaffold_tool.py WikipediaSearch`
2. Editing `WikipediaSearchTool.py` to add the `requests` logic and `query` argument schema.
How to Run this Example:
- Type:
Create a new ADK tool called StockPrice to fetch data from an API
. - Step 1 (Scaffolding): The agent runs the python script. This instantly creates
StockPriceTool.py
with the correct class structure, imports, and class nameStockPriceTool
. - Step 2 (Implementation): The agent "reads" the file it just made. It sees
# TODO: Implement logic.
- Step 3 (Guidance): It's not sure how to define the JSON schema for the tool arguments. It checks
examples/WeatherTool.py
. - Completion: It edits the file to add
requests.get(...)
and defines the ticker argument in the schema, exactly matching the ADK style.
6. Congratulations
You have successfully completed the lab on Antigravity Skills and built the following Skills:
- Git commit formatter.
- License header adder.
- JSON to Pydantic.
- Database schema validator.
- ADK Tool scaffolding.
Agent Skills are definitely a great way to bring Antigravity to write code in your way, follow rules, and use your tools.
Reference docs
- Codelab : Getting Started with Google Antigravity
- Official Site : https://antigravity.google/
- Documentation: https://antigravity.google/docs
- Download : https://antigravity.google/download
- Antigravity Skills documentation: https://antigravity.google/docs/skills

---

# skills/skills/oswalpalash/ontology/SKILL.md at main · openclaw/skills - GitHub
Source URL: https://github.com/openclaw/skills/blob/main/skills/oswalpalash/ontology/SKILL.md

Source Type: web_page

Source ID: 99e1122e-b80a-4174-a7d9-c9473a5e05d2


| name | ontology |
|---|---|
| description | Typed knowledge graph for structured agent memory and composable skills. Use when creating/querying entities (Person, Project, Task, Event, Document), linking related objects, enforcing constraints, planning multi-step actions as graph transformations, or when skills need to share state. Trigger on "remember", "what do I know about", "link X to Y", "show dependencies", entity CRUD, or cross-skill data access. |
A typed vocabulary + constraint system for representing knowledge as a verifiable graph.
Everything is an entity with a type, properties, and relations to other entities. Every mutation is validated against type constraints before committing.
Entity: { id, type, properties, relations, created, updated }
Relation: { from_id, relation_type, to_id, properties }
| Trigger | Action |
|---|---|
| "Remember that..." | Create/update entity |
| "What do I know about X?" | Query graph |
| "Link X to Y" | Create relation |
| "Show all tasks for project Z" | Graph traversal |
| "What depends on X?" | Dependency query |
| Planning multi-step work | Model as graph transformations |
| Skill needs shared state | Read/write ontology objects |
# Agents & People
Person: { name, email?, phone?, notes? }
Organization: { name, type?, members[] }
# Work
Project: { name, status, goals[], owner? }
Task: { title, status, due?, priority?, assignee?, blockers[] }
Goal: { description, target_date?, metrics[] }
# Time & Place
Event: { title, start, end?, location?, attendees[], recurrence? }
Location: { name, address?, coordinates? }
# Information
Document: { title, path?, url?, summary? }
Message: { content, sender, recipients[], thread? }
Thread: { subject, participants[], messages[] }
Note: { content, tags[], refs[] }
# Resources
Account: { service, username, credential_ref? }
Device: { name, type, identifiers[] }
Credential: { service, secret_ref } # Never store secrets directly
# Meta
Action: { type, target, timestamp, outcome? }
Policy: { scope, rule, enforcement }
Default: memory/ontology/graph.jsonl
{"op":"create","entity":{"id":"p_001","type":"Person","properties":{"name":"Alice"}}}
{"op":"create","entity":{"id":"proj_001","type":"Project","properties":{"name":"Website Redesign","status":"active"}}}
{"op":"relate","from":"proj_001","rel":"has_owner","to":"p_001"}
Query via scripts or direct file ops. For complex graphs, migrate to SQLite.
When working with existing ontology data or schema, append/merge changes instead of overwriting files. This preserves history and avoids clobbering prior definitions.
python3 scripts/ontology.py create --type Person --props '{"name":"Alice","email":"alice@example.com"}'
python3 scripts/ontology.py query --type Task --where '{"status":"open"}'
python3 scripts/ontology.py get --id task_001
python3 scripts/ontology.py related --id proj_001 --rel has_task
python3 scripts/ontology.py relate --from proj_001 --rel has_task --to task_001
python3 scripts/ontology.py validate # Check all constraints
Define in memory/ontology/schema.yaml
:
types:
Task:
required: [title, status]
status_enum: [open, in_progress, blocked, done]
Event:
required: [title, start]
validate: "end >= start if end exists"
Credential:
required: [service, secret_ref]
forbidden_properties: [password, secret, token] # Force indirection
relations:
has_owner:
from_types: [Project, Task]
to_types: [Person]
cardinality: many_to_one
blocks:
from_types: [Task]
to_types: [Task]
acyclic: true # No circular dependencies
Skills that use ontology should declare:
# In SKILL.md frontmatter or header
ontology:
reads: [Task, Project, Person]
writes: [Task, Action]
preconditions:
- "Task.assignee must exist"
postconditions:
- "Created Task has status=open"
Model multi-step plans as a sequence of graph operations:
Plan: "Schedule team meeting and create follow-up tasks"
1. CREATE Event { title: "Team Sync", attendees: [p_001, p_002] }
2. RELATE Event -> has_project -> proj_001
3. CREATE Task { title: "Prepare agenda", assignee: p_001 }
4. RELATE Task -> for_event -> event_001
5. CREATE Task { title: "Send summary", assignee: p_001, blockers: [task_001] }
Each step is validated before execution. Rollback on constraint violation.
Log ontology mutations as causal actions:
# When creating/updating entities, also log to causal action log
action = {
"action": "create_entity",
"domain": "ontology",
"context": {"type": "Task", "project": "proj_001"},
"outcome": "created"
}
# Email skill creates commitment
commitment = ontology.create("Commitment", {
"source_message": msg_id,
"description": "Send report by Friday",
"due": "2026-01-31"
})
# Task skill picks it up
tasks = ontology.query("Commitment", {"status": "pending"})
for c in tasks:
ontology.create("Task", {
"title": c.description,
"due": c.due,
"source": c.id
})
# Initialize ontology storage
mkdir -p memory/ontology
touch memory/ontology/graph.jsonl
# Create schema (optional but recommended)
python3 scripts/ontology.py schema-append --data '{
"types": {
"Task": { "required": ["title", "status"] },
"Project": { "required": ["name"] },
"Person": { "required": ["name"] }
}
}'
# Start using
python3 scripts/ontology.py create --type Person --props '{"name":"Alice"}'
python3 scripts/ontology.py list --type Person
references/schema.md
— Full type definitions and constraint patternsreferences/queries.md
— Query language and traversal examples
Runtime instructions operate on local files (memory/ontology/graph.jsonl
and memory/ontology/schema.yaml
) and provide CLI usage for create/query/relate/validate; this is within scope. The skill reads/writes workspace files and will create the memory/ontology
directory when used. Validation includes property/enum/forbidden checks, relation type/cardinality validation, acyclicity for relations marked acyclic: true
, and Event end >= start
checks; other higher-level constraints may still be documentation-only unless implemented in code.

---

# GitHub - guanyang/antigravity-skills: Empower agents with professional capabilities in specific fields (such as full-stack development, complex logic planning, multimedia processing, etc.) through modular Skills definitions, allowing agents to solve complex problems systematically like human experts.
Source URL: https://github.com/guanyang/antigravity-skills

Source Type: web_page

Source ID: 9d61649d-e8a7-4189-9e4c-e6e4583160b4


Empower agents with professional capabilities in specific fields (such as full-stack development, complex logic planning, multimedia processing, etc.) through modular Skills definitions, allowing agents to solve complex problems systematically like human experts.
.
├── .claude-plugin/ # Claude plugin configuration files
├── skills/ # Antigravity Skills library
│ ├── skill-name/ # Individual skill directory
│ │ ├── SKILL.md # Core skill definition and Prompt (Required)
│ │ ├── scripts/ # Scripts relied upon by the skill (Optional)
│ │ ├── examples/ # Skill usage examples (Optional)
│ │ └── resources/ # Templates and resources relied upon by the skill (Optional)
├── docs/ # User manual and documentation guides
├── scripts/ # Maintenance scripts
├── skills_sources.json # Skills synchronization source config
├── skills_index.json # Skills metadata index
├── spec/ # Specification documents
├── template/ # New skill template
└── README.md
Antigravity Skills follow the universal SKILL.md format and can work seamlessly with any AI coding assistant that supports Agentic Skills:
| Tool Name (Agent) | Type | Compatibility | Project Path | Global Path |
|---|---|---|---|---|
| Antigravity | IDE | ✅ Full | .agent/skills/ |
~/.gemini/antigravity/skills/ |
| Claude Code | CLI | ✅ Full | .claude/skills/ |
~/.claude/skills/ |
| Gemini CLI | CLI | ✅ Full | .gemini/skills/ |
~/.gemini/skills/ |
| Codex | CLI | ✅ Full | .codex/skills/ |
~/.codex/skills/ |
| Cursor | IDE | ✅ Full | .cursor/skills/ |
~/.cursor/skills/ |
| GitHub Copilot | Extension | .github/skills/ |
~/.copilot/skills/ |
|
| OpenCode | CLI | ✅ Full | .opencode/skills/ |
~/.config/opencode/skills/ |
| Windsurf | IDE | ✅ Full | .windsurf/skills/ |
~/.codeium/windsurf/skills/ |
| Trae | IDE | ✅ Full | .trae/skills/ |
~/.trae/skills/ |
Tip
Most tools will automatically discover skills in .agent/skills/
. For maximum compatibility, please clone/copy into this directory.
First, clone this repository locally (it is recommended to place it in a fixed location for global reference):
git clone https://github.com/guanyang/antigravity-skills.git ~/antigravity-skills
We strongly recommend using Symbolic Links (Symlink) for installation, so that when you update this repository via git pull
, all tools will automatically sync the latest features.
Enable skills only for the current project. Run in your project root:
mkdir -p .agent/skills
ln -s ~/antigravity-skills/skills/* .agent/skills/
Enable skills by default in all projects. Run the corresponding command based on the tool; common examples:
| Tool Name | Global Installation Command (macOS/Linux) |
|---|---|
| General | mkdir -p ~/.agent/skills && ln -s ~/antigravity-skills/skills/* ~/.agent/skills/ |
| Claude Code | mkdir -p ~/.claude/skills && ln -s ~/antigravity-skills/skills/* ~/.claude/skills/ |
| Antigravity | mkdir -p ~/.gemini/antigravity/skills && ln -s ~/antigravity-skills/skills/* ~/.gemini/antigravity/skills/ |
| Gemini | mkdir -p ~/.gemini/skills && ln -s ~/antigravity-skills/skills/* ~/.gemini/skills/ |
| Codex | mkdir -p ~/.codex/skills && ln -s ~/antigravity-skills/skills/* ~/.codex/skills/ |
If you primarily use Claude Code, you can install with one click via the plugin marketplace (this method automatically handles skill loading):
# 1. Start Claude Code
# 2. Add the plugin marketplace
/plugin marketplace add guanyang/antigravity-skills
# 3. Install the plugin from the marketplace
/plugin install antigravity-skills@antigravity-skills
Enter @[skill-name]
or /skill-name
in the chat box to invoke them, for example:
/canvas-design Help me design a 16:9 blog cover about "Deep Learning"
- View Manual: For detailed usage, please refer to docs/Antigravity_Skills_Manual.en.md.
- Environment Dependencies: Some skills rely on Python environments; please ensure your system has necessary libraries installed (e.g.,
pdf2docx
,pandas
, etc.).
Many skills in this project originate from excellent open-source communities. To keep in sync with upstream repositories, you can update them in the following ways:
-
Configuration: The
skills_sources.json
file in the root directory is pre-configured with the upstream repositories for major skills and usually does not need manual adjustment. -
Run Sync: You can choose to sync all skills or just a specific one:
# Sync all configured sources ./scripts/sync_skills.sh # Sync only a specific source (e.g., anthropics-skills) ./scripts/sync_skills.sh anthropics-skills
The script will automatically pull the latest code and update the corresponding skill directories.
Note: The
ui-ux-pro-max
skill has a special directory structure and does not support automatic synchronization via script for now. Please use its official installation commanduipro init --ai antigravity
to install or update.
These skills focus on visual expression, UI/UX design, and artistic creation.
@[algorithmic-art]
: Create algorithmic and generative art using p5.js code.@[canvas-design]
: Create posters and artworks (PNG/PDF output) based on design philosophies.@[json-canvas]
: Create and edit JSON Canvas files (.canvas
) with nodes, edges, and groups (commonly used in Obsidian).@[frontend-design]
: Create high-quality, production-grade frontend interfaces and Web components.@[ui-ux-pro-max]
: Professional UI/UX design intelligence, providing full design schemes for colors, fonts, layouts, etc.@[web-artifacts-builder]
: Build complex, modern Web apps (based on React, Tailwind, Shadcn/ui).@[theme-factory]
: Generate matching themes for documents, slides, HTML, etc.@[brand-guidelines]
: Apply Anthropic's official brand design specifications (colors, typography, etc.).@[remotion]
: Best practices for Remotion - Video creation in React.@[web-design-guidelines]
: Review UI code for Web Interface Guidelines compliance (accessibility, UX, design audit).@[slack-gif-creator]
: Create high-quality animated GIFs optimized specifically for Slack.
These skills cover the full lifecycle of coding, testing, debugging, and code review.
@[composition-patterns]
: React composition patterns for building scalable, flexible component libraries.@[react-best-practices]
: Vercel's official React and Next.js performance optimization guidelines.@[react-native-skills]
: React Native and Expo best practices for performant mobile apps.@[supabase-postgres-best-practices]
: Postgres performance optimization and best practices from Supabase.@[test-driven-development]
: Test-Driven Development (TDD) - write tests before implementation code.@[systematic-debugging]
: Systematic debugging for resolving bugs, test failures, or abnormal behaviors.@[webapp-testing]
: Use Playwright for interactive testing and verification of local web applications.@[receiving-code-review]
: Handle code review feedback using technical verification rather than blind modification.@[requesting-code-review]
: Proactively initiate code reviews to verify code quality before merging or completion.@[finishing-a-development-branch]
: Guide the finalization of a development branch (merges, PRs, cleanups, etc.).@[subagent-driven-development]
: Coordinate multiple sub-agents to perform independent development tasks in parallel.
These skills are used for handling professional documents and office needs in various formats.
@[doc-coauthoring]
: Guide users through collaborative writing of structured documents (proposals, tech specs, etc.).@[obsidian-markdown]
: Create and edit Obsidian Flavored Markdown with wikilinks, embeds, callouts, and properties.@[obsidian-bases]
: Create and edit Obsidian Bases (.base
) files with views, filters, formulas, and summaries.@[obsidian-cli]
: Interact with Obsidian vaults using the Obsidian CLI to read, create, search, and manage notes from the command line.@[defuddle]
: Extract clean markdown content from web pages using Defuddle CLI, removing clutter and navigation.@[docx]
: Create, edit, and analyze Word documents.@[xlsx]
: Create, edit, and analyze Excel spreadsheets (supporting formulas and charts).@[pptx]
: Create and modify PowerPoint presentations.@[pdf]
: Process PDF documents, including extracting text/tables, merging/splitting, and filling forms.@[internal-comms]
: Draft various corporate internal communication documents (weekly reports, announcements, FAQs, etc.).@[notebooklm]
: Query Google NotebookLM notebooks for definitive, document-grounded answers.
These skills help optimize workflows, task planning, and execution efficiency.
@[brainstorming]
: Brainstorm before starting any work to clarify requirements and design.@[writing-plans]
: Write detailed execution plans (Specs) for complex multi-step tasks.@[planning-with-files]
: A file-based planning system (Manus-style) suitable for complex tasks.@[executing-plans]
: Execute existing implementation plans with checkpoints and review mechanisms.@[using-git-worktrees]
: Create isolated Git worktrees for parallel development or task switching.@[verification-before-completion]
: Run verification commands to ensure concrete evidence before declaring task completion.@[using-superpowers]
: Guide users to discover and use these advanced skills.
These skills build the agent's mental models, memory systems, and context management capabilities.
@[bdi-mental-states]
: Simulate Agent's Belief-Desire-Intention (BDI) models.@[memory-systems]
: Build long-term memory and entity tracking systems based on knowledge graphs or vectors.@[context-fundamentals]
: Understand and debug fundamental issues like context windows and attention mechanisms.@[context-optimization]
: Optimize context efficiency to reduce Token costs via KV-cache or partitioning.@[context-compression]
: Implement context compression and summarization to handle long window limits.@[context-degradation]
: Diagnose and fix context degradation issues like "lost in the middle".@[filesystem-context]
: Utilize the filesystem for dynamic context offloading and management.
These skills focus on architectural design, tool building, and quality assessment of AI systems.
@[project-development]
: Full lifecycle design of LLM projects, including task-model matching and pipeline architecture.@[tool-design]
: Design efficient and clear agent tool interfaces and MCP protocols.@[evaluation]
: Establish multi-dimensional agent performance evaluation systems and quality gates.@[advanced-evaluation]
: Implement advanced evaluation methods like LLM-as-a-Judge and pairwise comparison.
These skills allow me to extend my own capability boundaries.
@[mcp-builder]
: Build MCP (Model Context Protocol) servers to connect external tools and data.@[skill-creator]
: Create new skills or update existing ones to expand my knowledge base and workflows.@[writing-skills]
: A subset of tools to assist in writing, editing, and verifying skill files.@[dispatching-parallel-agents]
: Dispatch parallel tasks to multiple agents for processing.@[multi-agent-patterns]
: Design advanced multi-agent collaboration patterns like Supervisor or Swarm.@[hosted-agents]
: Build and deploy sandboxed, persistently running background agents.
This project integrates core ideas or skill implementations from the following excellent open-source projects. Respect to the original authors:
- Anthropic Skills: Official API usage paradigms and skill definition references provided by Anthropic.
- UI/UX Pro Max Skills: Top-tier UI/UX design intelligence, providing full design schemes for colors, layouts, etc.
- Superpowers: A toolkit and workflow inspiration aimed at giving LLMs "superpowers."
- Planning with Files: Implements a Manus-style file-based task planning system to enhance persistent memory for complex tasks.
- NotebookLM: Knowledge retrieval and Q&A skill implementation based on Google NotebookLM.
- Agent-Skills-for-Context-Engineering: In-depth Context Engineering skills covering compression, optimization, and degradation handling.
- Obsidian Skills: Professional Obsidian integration skills, including JSON Canvas and enhanced Markdown support.
- Remotion Skills: Official Remotion skills for AI agents to create videos programmatically.
- Vercel Agent Skills: Official Vercel skills for React best practices, composition patterns, and web design guidelines.
- Supabase Agent Skills: Official Supabase skills for Postgres performance optimization and best practices.
We take security seriously. Please refer to our Security Policy for information on supported versions and how to report vulnerabilities safely.
We welcome contributions! Please refer to our CONTRIBUTING.md for detailed guidelines on how to add new skills, improve documentation, and report issues.
This project is open-sourced under the MIT License.

---

# sickn33/antigravity-awesome-skills: The Ultimate Collection ... - GitHub
Source URL: https://github.com/sickn33/antigravity-awesome-skills

Source Type: web_page

Source ID: d9134034-0a8d-4bcb-8ccc-9dd562147c4d


🌌 Antigravity Awesome Skills: 1,431+ Agentic Skills for Claude Code, Gemini CLI, Cursor, Copilot & More
Installable GitHub library of 1,431+ agentic skills for Claude Code, Cursor, Codex CLI, Gemini CLI, Antigravity, and other AI coding assistants.
Antigravity Awesome Skills is an installable GitHub library and npm installer for reusable SKILL.md
playbooks. It is designed for Claude Code, Cursor, Codex CLI, Gemini CLI, Antigravity, Kiro, OpenCode, GitHub Copilot, and other AI coding assistants that benefit from structured operating instructions. Instead of collecting one-off prompt snippets, this repository gives you a searchable, installable catalog of skills, bundles, workflows, plugin-safe distributions, and practical docs that help agents perform recurring tasks with better context, stronger constraints, and clearer outputs.
You can use this repo to install a broad multi-tool skill library, start from role-based bundles, or jump into workflow-driven execution for planning, coding, debugging, testing, security review, infrastructure, product work, and growth tasks. The root README is intentionally a high-signal landing page: understand what the project is, install it quickly, choose the right tool path, and then follow deeper docs only when you need them.
Start here: Star the repo · Install in 1 minute · Choose your tool · Best skills by tool · 📚 Browse 1,431+ Skills · Bundles · Workflows · Plugins for Claude Code and Codex
Current release: V10.5.0. Trusted by 34k+ GitHub stargazers, this repository combines official and community skill collections with bundles, workflows, installation paths, and docs that help you go from first install to daily use quickly.
- Installable, not just inspirational: use
npx antigravity-awesome-skills
to put skills where your tool expects them. - Built for major agent workflows: Claude Code, Cursor, Codex CLI, Gemini CLI, Antigravity, Kiro, OpenCode, Copilot, and more.
- Broad coverage with real utility: 1,431+ skills across development, testing, security, infrastructure, product, and marketing.
- Faster onboarding: bundles and workflows reduce the time from "I found this repo" to "I used my first skill".
- Useful whether you want breadth or curation: browse the full catalog, start with top bundles, or compare alternatives before installing.
- Why This Repo
- Installation
- Choose Your Tool
- Quick FAQ
- Best Skills By Tool
- Bundles & Workflows
- Browse 1,409+ Skills
- Troubleshooting
- Support the Project
- Contributing
- Community
- Credits & Sources
- Repo Contributors
- Star History
- License
Most users should start with the full library install and use bundles or workflows to narrow down what to try first.
# Default: ~/.gemini/antigravity/skills (Antigravity global). Use --path for other locations.
npx antigravity-awesome-skills
The npm installer uses a shallow clone by default so first-run installs stay lighter than a full repository history checkout.
test -d ~/.gemini/antigravity/skills && echo "Skills installed in ~/.gemini/antigravity/skills"
Use @brainstorming to plan a SaaS MVP.
- Use the full library install when you want the broadest catalog and direct control over your installed skills directory.
- Use the plugin route when you want a marketplace-style, plugin-safe distribution for Claude Code or Codex.
- Read Plugins for Claude Code and Codex for the full breakdown of full-library install vs plugin install vs bundle plugins.
Use the same repository, but install or invoke it in the way your host expects.
| Tool | Install | First Use |
|---|---|---|
| Claude Code | npx antigravity-awesome-skills --claude or Claude plugin marketplace |
>> /brainstorming help me plan a feature |
| Cursor | npx antigravity-awesome-skills --cursor |
@brainstorming help me plan a feature |
| Gemini CLI | npx antigravity-awesome-skills --gemini |
Use brainstorming to plan a feature |
| Codex CLI | npx antigravity-awesome-skills --codex |
Use brainstorming to plan a feature |
| Antigravity | npx antigravity-awesome-skills --antigravity |
Use @brainstorming to plan a feature |
| Kiro CLI | npx antigravity-awesome-skills --kiro |
Use brainstorming to plan a feature |
| Kiro IDE | npx antigravity-awesome-skills --path ~/.kiro/skills |
Use @brainstorming to plan a feature |
| GitHub Copilot | No installer — paste skills or rules manually | Ask Copilot to use brainstorming to plan a feature |
| OpenCode | npx antigravity-awesome-skills --path .agents/skills --category development,backend --risk safe,none |
opencode run @brainstorming help me plan a feature |
| AdaL CLI | npx antigravity-awesome-skills --path .adal/skills |
Use brainstorming to plan a feature |
| Custom path | npx antigravity-awesome-skills --path ./my-skills |
Depends on your tool |
For path details, prompt examples, and setup caveats by host, go to:
It is an installable GitHub library of reusable SKILL.md
playbooks for Claude Code, Cursor, Codex CLI, Gemini CLI, Antigravity, and related AI coding assistants. The repo packages those skills with an installer CLI, bundles, workflows, generated catalogs, and docs so you can move from discovery to daily usage quickly.
Run npx antigravity-awesome-skills
for the default full-library install, or use a tool-specific flag such as --codex
, --cursor
, --gemini
, --claude
, or --antigravity
when you want the installer to target a known skills directory directly.
Use the full library if you want the biggest catalog and direct filesystem control. Use plugins when you want a marketplace-style, plugin-safe distribution for Claude Code or Codex. The complete explanation lives in Plugins for Claude Code and Codex.
Start with Bundles for role-based recommendations, Workflows for ordered execution playbooks, CATALOG.md for the full registry, and the hosted GitHub Pages catalog when you want a browsable web UI.
If you want a faster answer than "browse all 1,431+ skills", start with a tool-specific guide:
- Claude Code skills: install paths, starter skills, prompt examples, and plugin marketplace flow.
- Cursor skills: best starter skills for
.cursor/skills/
, UI-heavy work, and pair-programming flows. - Codex CLI skills: planning, implementation, debugging, and review skills for local coding loops.
- Gemini CLI skills: starter stack for research, agent systems, integrations, and engineering workflows.
- AI agent skills guide: how to evaluate skill libraries, choose breadth vs curation, and pick the right starting point.
@brainstorming
for planning before implementation.@test-driven-development
for TDD-oriented work.@debugging-strategies
for systematic troubleshooting.@lint-and-validate
for lightweight quality checks.@security-auditor
for security-focused reviews.@frontend-design
for UI and interaction quality.@api-design-principles
for API shape and consistency.@create-pr
for packaging work into a clean pull request.
Use @brainstorming to turn this product idea into a concrete MVP plan.
Use @security-auditor to review this API endpoint for auth and validation risks.
Bundles help you choose where to start. Workflows help you execute skills in the right order.
Bundles are curated groups of recommended skills for a role or goal such as Web Wizard
, Security Engineer
, or OSS Maintainer
.
- Bundles are recommendations, not separate installs.
- Install the repository once, then use docs/users/bundles.md to pick a starting set.
- Good starter combinations:
- SaaS MVP:
Essentials
+Full-Stack Developer
+QA & Testing
- Production hardening:
Security Developer
+DevOps & Cloud
+Observability & Monitoring
- OSS shipping:
Essentials
+OSS Maintainer
- SaaS MVP:
- Read docs/users/workflows.md for human-readable playbooks.
- Use data/workflows.json for machine-readable workflow metadata.
- Initial workflows include shipping a SaaS MVP, security audits, AI agent systems, QA/browser automation, and DDD-oriented design work.
If Antigravity starts hitting context limits with too many active skills, the activation guidance in docs/users/agent-overload-recovery.md can materialize only the bundles or skill ids you want in the live Antigravity directory.
If you use OpenCode or another .agents/skills
host, prefer a reduced install up front instead of copying the full library into a context-sensitive runtime. The installer now supports --risk
, --category
, and --tags
so you can keep the installed set narrow.
Use the root repo as a landing page, then jump into the deeper surface that matches your intent.
- Skills library in
skills/
- Installer CLI powered by the npm package in
package.json
- Generated catalog and metadata in
CATALOG.md
,skills_index.json
, anddata/
- Hosted and local web app in
apps/web-app
and on GitHub Pages - Role-based bundles in docs/users/bundles.md
- Execution workflows in docs/users/workflows.md
- User, contributor, and maintainer docs under
docs/
- Read the full catalog in
CATALOG.md
. - Browse the hosted catalog at https://sickn33.github.io/antigravity-awesome-skills/.
- Start with Getting Started and Usage if you are new after installation.
- Use Bundles for role-based discovery and Workflows for step-by-step execution.
- Use Plugins for Claude Code and Codex when you care about marketplace-safe distribution.
- Antigravity Awesome Skills vs Awesome Claude Skills for breadth vs curated-list tradeoffs.
- Best Claude Code skills on GitHub for a high-intent shortlist.
- Best Cursor skills on GitHub for Cursor-compatible options and selection criteria.
Keep the root README short; use the dedicated docs for recovery and platform-specific guidance.
- If you are confused after installation, start with the Usage Guide.
- For Windows truncation or context crash loops, use docs/users/windows-truncation-recovery.md.
- For Linux/macOS overload or selective activation, use docs/users/agent-overload-recovery.md.
- For OpenCode or other
.agents/skills
installs, prefer a reduced install such asnpx antigravity-awesome-skills --path .agents/skills --category development,backend --risk safe,none
. - For plugin install details, host compatibility, and marketplace-safe distribution, use docs/users/plugins.md.
- For contributor expectations and guardrails, use CONTRIBUTING.md,
CODE_OF_CONDUCT.md
, andSECURITY.md
.
Support is optional. The project stays free and open-source for everyone.
- Buy me a book on Buy Me a Coffee
- Star the repository
- Open reproducible issues
- Contribute docs, fixes, and skills
- Add new skills under
skills/<skill-name>/SKILL.md
. - Follow the contributor guide in
CONTRIBUTING.md
. - Use the template in
docs/contributors/skill-template.md
. - Validate with
npm run validate
before opening a PR. - Keep community PRs source-only: do not commit generated registry artifacts like
CATALOG.md
,skills_index.json
, ordata/*.json
. - If your PR changes
SKILL.md
, expect the automatedskill-review
check on GitHub in addition to the usual validation and security scans. - If your PR changes skills or risky guidance, manual logic review is still required even when the automated checks are green.
- Discussions for questions, ideas, showcase posts, and community feedback.
- Issues for reproducible bugs and concrete, actionable improvement requests.
- Follow @sickn33 on X for project updates and releases.
CODE_OF_CONDUCT.md
for community expectations and moderation standards.SECURITY.md
for security reporting.
We stand on the shoulders of giants.
👉 View the Full Attribution Ledger
Key contributors and sources include:
- HackTricks
- OWASP
- Anthropic / OpenAI / Google
- The Open Source Community
This collection would not be possible without the incredible work of the Claude Code community and official sources:
- anthropics/skills: Official Anthropic skills repository - Document manipulation (DOCX, PDF, PPTX, XLSX), Brand Guidelines, Internal Communications.
- anthropics/claude-cookbooks: Official notebooks and recipes for building with Claude.
- remotion-dev/skills: Official Remotion skills - Video creation in React with 28 modular rules.
- vercel-labs/agent-skills: Vercel Labs official skills - React Best Practices, Web Design Guidelines.
- openai/skills: OpenAI Codex skills catalog - Agent skills, Skill Creator, Concise Planning.
- supabase/agent-skills: Supabase official skills - Postgres Best Practices.
- microsoft/skills: Official Microsoft skills - Azure cloud services, Bot Framework, Cognitive Services, and enterprise development patterns across .NET, Python, TypeScript, Go, Rust, and Java.
- MiniMax-AI/cli: Official MiniMax CLI - text, image, video, speech, music, vision, and web-search workflows for MiniMax models and APIs.
- google-gemini/gemini-skills: Official Gemini skills - Gemini API, SDK and model interactions.
- apify/agent-skills: Official Apify skills - Web scraping, data extraction and automation.
- expo/skills: Official Expo skills - Expo project workflows and Expo Application Services guidance.
- huggingface/skills: Official Hugging Face skills - Models, Spaces, datasets, inference, and broader Hugging Face ecosystem workflows.
- neondatabase/agent-skills: Official Neon skills - Serverless Postgres workflows and Neon platform guidance.
- scopeblind/scopeblind-gateway: Official Scopeblind MCP governance toolkit - Cedar policy authoring, shadow-to-enforce rollout, and signed-receipt verification guidance for agent tool calls.
- monte-carlo-data/mc-agent-toolkit: Monte Carlo data observability skills — table health checks, change impact assessment, monitor creation, push ingestion, and SQL validation notebooks for dbt changes.
- openclaw/skills: Source for the
daily-gift
skill - relationship-aware creative gift generation with editorial judgment, concept selection, and multi-format rendering. - umutbozdag/agent-skills-manager: Source for the
manage-skills
skill - cross-tool skill discovery, creation, editing, toggling, copying, moving, and deletion workflows across major agent coding tools. - pumanitro/global-chat: Source for the Global Chat Agent Discovery skill - cross-protocol discovery of MCP servers and AI agents across multiple registries.
- bitjaru/styleseed: StyleSeed Toss UI and UX skill collection - setup wizard, page and pattern generation, design-token management, accessibility review, UX audits, feedback states, and microcopy guidance for professional mobile-first UI.
- milkomida77/guardian-agent-prompts: Source for the Multi-Agent Task Orchestrator skill - production-tested delegation patterns, anti-duplication, and quality gates for coordinated agent work.
- Elkidogz/technical-change-skill: Source for the Technical Change Tracker skill - structured JSON change records, session handoff, and accessible HTML dashboards for coding continuity.
- rmyndharis/antigravity-skills: For the massive contribution of 300+ Enterprise skills and the catalog generation logic.
- amartelr/antigravity-workspace-manager: Workspace Manager CLI companion to dynamically auto-provision subsets of skills across local development environments.
- obra/superpowers: The original "Superpowers" by Jesse Vincent.
- guanyang/antigravity-skills: Core Antigravity extensions.
- diet103/claude-code-infrastructure-showcase: Infrastructure and Backend/Frontend Guidelines.
- ChrisWiles/claude-code-showcase: React UI patterns and Design Systems.
- travisvn/awesome-claude-skills: Loki Mode and Playwright integration.
- Dimillian/Skills: Curated Codex skills focused on Apple platforms, GitHub workflows, refactoring, and performance (MIT).
- zebbern/claude-code-guide: Comprehensive Security suite & Guide (Source for ~60 new skills).
- alirezarezvani/claude-skills: Senior Engineering and PM toolkit.
- karanb192/awesome-claude-skills: A massive list of verified skills for Claude Code.
- VoltAgent/awesome-agent-skills: Curated collection of 1000+ official and community agent skills from leading development teams (MIT).
- zircote/.claude: Archived Claude Code dotfiles/config repo with a Shopify development skill reference.
- vibeforge1111/vibeship-spawner-skills: AI agents, integrations, maker tools, and other production-grade skill packs.
- coreyhaines31/marketingskills: Marketing skills for CRO, copywriting, SEO, paid ads, and growth (23 skills, MIT).
- AgriciDaniel/claude-seo: SEO workflow collection covering technical SEO, hreflang, sitemap, geo, schema, and programmatic SEO patterns.
- Leonxlnx/taste-skill: Frontend design taste skill collection covering premium UI generation, redesign audits, GSAP motion, Stitch design systems, minimalist and brutalist visual modes, and full-output enforcement.
- mrprewsh/seo-aeo-engine: SEO/AEO content-growth system covering keyword research, content clustering, landing pages, blog structure, schema, internal linking, and audit workflows.
- jonathimer/devmarketing-skills: Developer marketing skills — HN strategy, technical tutorials, docs-as-marketing, Reddit engagement, developer onboarding, and more (33 skills, MIT).
- kepano/obsidian-skills: Obsidian-focused skills for markdown, Bases, JSON Canvas, CLI workflows, and content cleanup.
- lewiswigmore/agent-skills: Source for the
vscode-extension-guide-en
skill - VS Code extension development workflows, packaging, Marketplace publishing, TreeView, and webview patterns. - Silverov/yandex-direct-skill: Yandex Direct (API v5) advertising audit skill — 55 automated checks, A-F scoring, campaign/ad/keyword analysis for the Russian PPC market (MIT).
- vudovn/antigravity-kit: AI Agent templates with Skills, Agents, and Workflows (33 skills, MIT).
- affaan-m/everything-claude-code: Large Claude Code configuration and workflow collection from an Anthropic hackathon winner (MIT).
- whatiskadudoing/fp-ts-skills: Practical fp-ts skills for TypeScript – fp-ts-pragmatic, fp-ts-react, fp-ts-errors (v4.4.0).
- warmskull/idea-darwin: Darwinian idea-evolution workflow for structured ideation rounds, mutation, crossbreeding, critique, and lineage tracking.
- Slashworks-biz/idea-os: Source for the
idea-os
skill - five-phase pipeline (triage -> clarify -> research -> PRD -> plan) that turns raw ideas into a build-ready PRD and execution plan. - webzler/agentMemory: Source for the agent-memory-mcp skill.
- rafsilva85/credit-optimizer-v5: Manus AI credit optimizer skill — intelligent model routing, context compression, and smart testing. Saves 30-75% on credits with zero quality loss. Audited across 53 scenarios.
- Wittlesus/cursorrules-pro: Professional .cursorrules configurations for 8 frameworks - Next.js, React, Python, Go, Rust, and more. Works with Cursor, Claude Code, and Windsurf.
- nedcodes-ok/rule-porter: Bidirectional rule converter between Cursor (.mdc), Claude Code (CLAUDE.md), GitHub Copilot, Windsurf, and legacy .cursorrules formats. Zero dependencies.
- SSOJet/skills: Production-ready SSOJet skills and integration guides for popular frameworks and platforms — Node.js, Next.js, React, Java, .NET Core, Go, iOS, Android, and more. Works seamlessly with SSOJet SAML, OIDC, and enterprise SSO flows. Works with Cursor, Antigravity, Claude Code, and Windsurf.
- MojoAuth/skills: Production-ready MojoAuth guides and examples for popular frameworks like Node.js, Next.js, React, Java, .NET Core, Go, iOS, and Android.
- Xquik-dev/x-twitter-scraper: X (Twitter) data platform — tweet search, user lookup, follower extraction, engagement metrics, giveaway draws, monitoring, webhooks, 19 extraction tools, MCP server.
- connerlambden/helium-mcp: Source for the
helium-mcp
skill — MCP server for news intelligence, media bias analysis, market data, options pricing, and semantic meme search. - shmlkv/dna-claude-analysis: Personal genome analysis toolkit — Python scripts analyzing raw DNA data across 17 categories (health risks, ancestry, pharmacogenomics, nutrition, psychology, etc.) with terminal-style single-page HTML visualization.
- AlmogBaku/debug-skill: Interactive debugger skill for AI agents — breakpoints, stepping, variable inspection, and stack traces via the
dap
CLI. Supports Python, Go, Node.js/TypeScript, Rust, and C/C++. - uberSKILLS: Design, test, and deploy Claude Code Agent Skills through a visual, AI-assisted workflow.
- christopherlhammer11-ai/tool-use-guardian: Source for the Tool Use Guardian skill — tool-call reliability wrapper with retries, recovery, and failure classification.
- christopherlhammer11-ai/recallmax: Source for the RecallMax skill — long-context memory, summarization, and conversation compression for agents.
- tsilverberg/webapp-uat: Full browser UAT skill — Playwright testing with console/network error capture, WCAG 2.2 AA accessibility checks, i18n validation, responsive testing, and P0-P3 bug triage. Read-only by default, works with React, Vue, Angular, Ionic, Next.js.
- Wolfe-Jam/faf-skills: AI-context and project DNA skills — .faf format management, AI-readiness scoring, bi-sync, MCP server building, and championship-grade testing (17 skills, MIT).
- fullstackcrew-alpha/privacy-mask: Local image privacy masking for AI coding agents. Detects and redacts PII, API keys, and secrets in screenshots via OCR + 47 regex rules. Claude Code hook integration for automatic masking. Supports Tesseract and RapidOCR. 100% offline (MIT).
- AvdLee/SwiftUI-Agent-Skill: SwiftUI best-practices skill for agent workflows (MIT).
- CloudAI-X/threejs-skills: Three.js-focused skill collection for agent-assisted 3D web work.
- K-Dense-AI/claude-scientific-skills: Scientific, research, engineering, finance, and writing skill suite (MIT).
- NotMyself/claude-win11-speckit-update-skill: Archived Speckit update skill for Claude Code (MIT).
- SHADOWPR0/beautiful_prose: Writing-quality skill for improving prose and reducing generic output.
- SHADOWPR0/security-bluebook-builder: Security documentation/buildbook skill for agent workflows.
- SeanZoR/claude-speed-reader: RSVP-style speed-reading helper for Claude responses (MIT).
- Shpigford/skills: General-purpose agent skills for common development tasks (MIT).
- ZhangHanDong/makepad-skills: Makepad app-development skills and references (MIT).
- czlonkowski/n8n-skills: n8n workflow-building skills for Claude Code (MIT).
- frmoretto/clarity-gate: Verification protocol for marking uncertainty and reducing hallucinated certainty in LLM-facing docs.
- fruitwyatt/puzzle-activity-planner: Puzzle activity-planning skill for classrooms, parties, and events with generator-link workflows.
- gokapso/agent-skills: Kapso/WhatsApp-oriented agent skills.
- huifer/WellAlly-health: Healthcare assistant project cited in release history as a source for health-focused agent capabilities (MIT).
- ibelick/ui-skills: UI-polish skills for improving interfaces built by agents (MIT).
- jackjin1997/ClawForge: Resource hub of skills, MCP servers, and agent tooling for OpenClaw.
- jthack/ffuf_claude_skill: FFUF skill for web fuzzing workflows in Claude.
- MetcalfSolutions/Satori: Clinically informed wisdom companion blending psychology frameworks and wisdom traditions into a structured reflective partner.
- muratcankoylan/Agent-Skills-for-Context-Engineering: Context-engineering, multi-agent, and production agent-system skill collection (MIT).
- robzolkos/skill-rails-upgrade: Rails upgrade skill for agent-assisted migrations.
- sanjay3290/ai-skills: Apache-licensed collection of agent skills for AI coding assistants.
- scarletkc/vexor: Semantic search engine for files and code, referenced in release history.
- sstklen/infinite-gratitude: Multi-agent research skill from the AI Dojo series (MIT).
- wrsmith108/linear-claude-skill: Linear issue/project/team management skill with MCP and GraphQL workflows (MIT).
- wrsmith108/varlock-claude-skill: Secure environment-variable management skill for Claude Code (MIT).
- zarazhangrui/frontend-slides: Frontend slide-creation skills for web-based presentations (MIT).
- zxkane/aws-skills: AWS-focused Claude agent skills (MIT).
- UrRhb/agentflow: Kanban-driven AI development pipeline for orchestrating multi-worker Claude Code workflows with deterministic quality gates, adversarial review, cost tracking, and crash-proof execution (MIT).
- AgentPhone-AI/skills: AgentPhone plugin for Claude Code — API-first telephony workflows for AI agents, including phone calls, SMS, phone-number management, voice-agent setup, streaming webhooks, and tool-calling patterns.
- uxuiprinciples/agent-skills: Research-backed UX/UI agent skills for auditing interfaces against 168 principles, detecting antipatterns, and injecting UX context into AI coding sessions.
- voidborne-d/humanize-chinese: Chinese AI-text detection and humanization toolkit for scoring, rewriting, academic AIGC reduction, and style conversion workflows.
- LambdaTest/agent-skills: Production-grade agent skills for test automation — 46 skills covering E2E, unit, mobile, BDD, visual, and cloud testing across 15+ languages (MIT).
- f/awesome-chatgpt-prompts: Inspiration for the Prompt Library.
- leonardomso/33-js-concepts: Inspiration for JavaScript Mastery.
- agent-cards/skill: Manage prepaid virtual Visa cards for AI agents. Create cards, check balances, view credentials, close cards, and get support via MCP tools.
Made with contrib.rocks. (Image may be cached; view live contributors on GitHub.)
We officially thank the following contributors for their help in making this repository awesome!
- @sck000
- @github-actions[bot]
- @sickn33
- @munir-abbasi
- @Mohammad-Faiz-Cloud-Engineer
- @zinzied
- @ssumanbiswas
- @Champbreed
- @Dokhacgiakhoa
- @sx4im
- @maxdml
- @IanJ332
- @skyruh
- @ar27111994
- @chauey
- @itsmeares
- @suhaibjanjua
- @GuppyTheCat
- @Copilot
- @8hrsk
- @fernandorych
- @nikolasdehor
- @SnakeEye-sudo
- @talesperito
- @zebbern
- @sstklen
- @0xrohitgarg
- @tejasashinde
- @jackjin1997
- @HuynhNhatKhanh
- @taksrules
- @liyin2015
- @fullstackcrew-alpha
- @dz3ai
- @fernandezbaptiste
- @Gizzant
- @JayeHarrill
- @AssassinMaeve
- @Musayrlsms
- @arathiesh
- @RamonRiosJr
- @Tiger-Foxx
- @TomGranot
- @truongnmt
- @UrRhb
- @uriva
- @babysor
- @code-vj
- @viktor-ferenczi
- @vprudnikoff
- @Vonfry
- @wahidzzz
- @vuth-dogo
- @terryspitz
- @Onsraa
- @SebConejo
- @SuperJMN
- @Enreign
- @sohamganatra
- @Silverov
- @conspirafi
- @shubhamdevx
- @ronanguilloux
- @sraphaz
- @ProgramadorBrasil
- @prewsh
- @PabloASMD
- @yubing744
- @hazemezz123
- @yang1002378395-cmyk
- @viliawang-pm
- @uucz
- @tsilverberg
- @thuanlm215
- @shmlkv
- @rafsilva85
- @nocodemf
- @marsiandeployer
- @ksgisang
- @KrisnaSantosa15
- @kostakost2
- @junited31
- @fbientrigo
- @developer-victor
- @ckdwns9121
- @dependabot[bot]
- @christopherlhammer11-ai
- @c1c3ru
- @buzzbysolcex
- @BenZinaDaze
- @avimak
- @antbotlab
- @amalsam
- @ziuus
- @Wolfe-Jam
- @jamescha-earley
- @ivankoriako
- @rcigor
- @hvasconcelos
- @Guilherme-ruy
- @FrancyJGLisboa
- @framunoz
- @Digidai
- @dbhat93
- @decentraliser
- @MAIOStudio
- @wd041216-bit
- @conorbronsdon
- @RoundTable02
- @ChaosRealmsAI
- @kriptoburak
- @BenedictKing
- @acbhatt12
- @Andruia
- @AlmogBaku
- @Allen930311
- @alexmvie
- @Sayeem3051
- @Abdulrahmansoliman
- @ALEKGG1
- @8144225309
- @sharmanilay
- @KhaiTrang1995
- @LocNguyenSGU
- @nedcodes-ok
- @MMEHDI0606
- @iftikharg786
- @halith-smh
- @mertbaskurt
- @modi2meet
- @MatheusCampagnolo
- @donbagger
- @Marvin19700118
- @djmahe4
- @MArbeeGit
- @majorelalexis-stack
- @Svobikl
- @kromahlusenii-ops
- @Krishna-Modi12
- @k-kolomeitsev
- @kennyzheng-builds
- @keyserfaty
- @kage-art
- @whatiskadudoing
- @joselhurtado
- @jonathimer
- @Jonohobs
- @JaskiratAnand
- @Al-Garadi
- @olgasafonova
- @Elkidogz
- @qcwssss
- @spideyashith
- @tomjwxf
- @Cerdore
- @MetcalfSolutions
- @warmskull
- @Wittlesus
- @digitamaz
- @cryptoque
- @umutbozdag
- @hqhq1025
- @htafolla
- @playbookTV
- @derricke
- @sebastiondev
- @WHOISABHISHEKADHIKARI
- @HMAKT99
- @nickdesi
- @connerlambden
- @zhangyanxs
- @818cortex
- @octo-patch
- @fruitwyatt
- @jiawei248
- @tanveer-farooq
- @emanoelCarvalho
- @unitedideas
- @globalchatapp
- @edudeftones-cloud
- @Evozim
- @Imasaikiran
- @justmiroslav
- @1bcMax
- @xiaolai
If Antigravity Awesome Skills has been useful, consider ⭐ starring the repo!
Original code and tooling are licensed under the MIT License. See LICENSE.
Original documentation and other non-code written content are licensed under CC BY 4.0, unless a more specific upstream notice says otherwise. See docs/sources/sources.md for attributions and third-party license details.

---

