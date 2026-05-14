import sqlite3

conn = sqlite3.connect("soda_heuristic_vault.db")
conn.row_factory = sqlite3.Row

rows = conn.execute(
    "SELECT repo_id, classificacao_terminal, score_total, "
    "score_arquitetura, score_rust_potential, score_bare_metal, "
    "categoria_arquitetural, "
    "substr(lente_a_sentido_ux,    1, 90) AS lente_a, "
    "substr(lente_b_estrutura_arq, 1, 90) AS lente_b, "
    "substr(lente_c_realidade_ops, 1, 80) AS lente_c, "
    "substr(executive_verdict,     1,110) AS verdict "
    "FROM repo_heuristics "
    "WHERE lote_id='LOTE_PILOT_01' "
    "ORDER BY score_total DESC"
).fetchall()

print(f"\n{'='*70}")
print(f"  VAULT AUDIT -- LOTE_PILOT_01 -- {len(rows)} registros")
print(f"{'='*70}")

for r in rows:
    print()
    print(f"  repo_id              : {r['repo_id']}")
    print(f"  classificacao        : {r['classificacao_terminal']}")
    print(f"  score_total          : {r['score_total']}")
    print(f"  score_arquitetura    : {r['score_arquitetura']}")
    print(f"  score_rust_potential : {r['score_rust_potential']}")
    print(f"  score_bare_metal     : {r['score_bare_metal']}")
    print(f"  categoria            : {r['categoria_arquitetural']}")
    print(f"  lente_A (UX)         : {r['lente_a']}...")
    print(f"  lente_B (Arq)        : {r['lente_b']}...")
    print(f"  lente_C (Ops)        : {r['lente_c'] or 'N/A (falha)'}")
    print(f"  verdict              : {r['verdict']}")

run = conn.execute(
    "SELECT status, repos_ok, repos_erro, iniciado_em, finalizado_em "
    "FROM etl_run_log ORDER BY iniciado_em DESC LIMIT 1"
).fetchone()

print(f"\n{'='*70}")
print("  RUN LOG (ultimo)")
print(f"{'='*70}")
print(f"  status      : {run['status']}")
print(f"  repos_ok    : {run['repos_ok']}")
print(f"  repos_erro  : {run['repos_erro']}")
print(f"  iniciado_em : {run['iniciado_em']}")
print(f"  finalizado  : {run['finalizado_em']}")
print(f"{'='*70}\n")

conn.close()
