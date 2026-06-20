Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "[SODA] Arsenal SAST da Fabrica (Windows host)" -ForegroundColor Cyan
Write-Host "[SODA] Script de reproducao manual, sem upgrades forcados nem validacao sincrona." -ForegroundColor DarkGray

# Elixir + Erlang
winget install Erlang.Erlang Elixir.Elixir

# Phoenix / Elixir SAST
mix archive.install hex sobelow

# Go
go install golang.org/x/vuln/cmd/govulncheck@latest

# Python
uv tool install ruff
uv tool install bandit

# JS / TS / Frontend
npm install -g @biomejs/biome oxlint

# opengrep:
# Baixar o binario Windows na Release oficial do GitHub e mover manualmente
# `opengrep.exe` para `$HOME\.cargo\bin\opengrep.exe` para expor no PATH da Fabrica.
