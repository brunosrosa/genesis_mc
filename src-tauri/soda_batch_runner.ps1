# ==============================================================================
# SODA ETL V3 - ORQUESTRADOR DE LOTES BARE-METAL (JANELA DE VIDRO)
# ==============================================================================
 
# 1. DEFINA O NOME DO LOTE
$BATCH_ID = "LOTE_03_FINANCE_E_AGENTES"
 
# 2. LISTE OS REPOSITÓRIOS DO LOTE ATUAL
$REPOS = @(
    "tinyhumansai/openhuman",

)
 
Clear-Host
Write-Host "=====================================================" -ForegroundColor Cyan
Write-Host " 🦅 SODA ETL V3 - PRE-FLIGHT CHECK (JANELA DE VIDRO)" -ForegroundColor White -BackgroundColor DarkBlue
Write-Host "=====================================================" -ForegroundColor Cyan
Write-Host " Lote Alvo   : $BATCH_ID" -ForegroundColor Yellow
Write-Host " Total Repos : $($REPOS.Count)" -ForegroundColor Yellow
Write-Host "-----------------------------------------------------" -ForegroundColor Cyan
Write-Host " Fila de Ingestão:" -ForegroundColor White
foreach ($repo in $REPOS) {
    Write-Host "  -> $repo" -ForegroundColor Gray
}
Write-Host "=====================================================" -ForegroundColor Cyan
 
# Formatar a lista para o Python
$repoListStr = '"' + ($REPOS -join '", "') + '"'
 
# Script Python injetado atamicamente na RAM para bypass do SQLite
$pythonScript = @"
import sqlite3
import os
 
repos_lote = [$repoListStr]
lote_id = '$BATCH_ID'
 
db_path = os.path.join('.soda_data', 'soda_heuristic_vault.db')
conn = sqlite3.connect(db_path)
 
for repo in repos_lote:
    sql = f\"\"\"
    INSERT INTO repositorios (project_name, repo_url, status_processamento, lote_id, soda_universal_uuid) 
    VALUES ('{repo}', ' `https://github.com/{repo}` ', 'PENDENTE', '{lote_id}', hex(randomblob(16)))
    ON CONFLICT(project_name) DO UPDATE SET 
    status_processamento = 'PENDENTE',
    lote_id = '{lote_id}';
    \"\"\"
    conn.execute(sql)
 
conn.commit()
conn.close()
"@
 
Write-Host "`n[1/3] Sincronizando com o Cérebro SQLite (Fase -1)..." -ForegroundColor Green
New-Item -ItemType Directory -Force -Path ".soda_data" | Out-Null
$pythonScript | python -
Write-Host "[2/3] Memória de estados atualizada com sucesso!" -ForegroundColor Green
 
Write-Host "`n=====================================================" -ForegroundColor Cyan
# HITL: Ponto de Veto Humano
$confirmation = Read-Host "🔥 Arquiteto, autoriza a IGNICÃO do Trator ETL Cognitivo E2E? (S/N)"
 
if ($confirmation -match "^[sS]$") {
    Write-Host "`n[3/3] DISPARANDO O MOTOR EM RUST! (Zero-AI para Fase 1 -> DeepSeek Fase 3)...`n" -ForegroundColor Red
    cargo run -q --bin f3_synthesizer_cli -- --e2e-full
} else {
    Write-Host "`n🛑 Execução ABORTADA pelo Arquiteto." -ForegroundColor Yellow
    Write-Host "Os repositórios estão marcados como 'PENDENTE' no banco. Você pode rodar o cargo run no futuro." -ForegroundColor Gray
}
