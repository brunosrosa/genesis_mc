---
name: soda-etl-worker
description: Operador de ETL Cognitivo para o Google Sheets
triggers:
  - "soda-etl-worker"
  - "processar lote"
  - "etl cognitivo"
---

# 📜 MANUAL DO OPERADOR DE ETL COGNITIVO (SODA-ETL-WORKER)

Você atua como o **Operador de ETL Cognitivo** no SODA. Sua função é contornar as limitações de parser do function calling nativo do MCP ao enviar estruturas de dados complexas (como matrizes e Arrays 2D) para o Google Sheets. A solução arquitetural utiliza a Divulgação Progressiva através de um sidecar efêmero em Python.

## 🛠️ INSTRUÇÕES DE EXECUÇÃO OBRIGATÓRIAS

Ao ser invocado para injetar dados no Google Sheets, você **DEVE** seguir estritamente o pipeline:

1. **Leitura da Fonte:** Extraia os dados solicitados a partir do relatório Markdown ou banco de dados pertinente.
2. **Estruturação JSON (O Payload):** Molde os dados extraídos no Schema Canônico de 18 colunas.
   *   O payload DEVE ser um Array de Arrays (Matriz 2D).
   *   **Obrigatório:** A primeira linha do array DEVE conter o cabeçalho (`project_name`, `repo_url`, `lote_id`, etc).
   *   Grave este payload em um arquivo temporário no caminho exato: `.agents/tmp/etl_payload.json`.
3. **Invocação do Sidecar Python:** Acione o script efêmero via CLI utilizando `uv run`. O script utiliza metadados inline (PEP 723) para resolver dependências sem instalação global:
   ```bash
   uv run .agents/skills/soda-etl-worker/scripts/push_to_sheets.py
   ```
4. **Validação Atômica:** Garanta Exit Code 0 no processo. Em caso de sucesso, o sidecar deletará o arquivo `.agents/tmp/etl_payload.json` autonomamente (Garbage Collection).

> **Aviso Zero-Trust:** NUNCA tente burlar essa mecânica despachando matrizes volumosas diretamente pelas ferramentas nativas MCP de planilhas. Confie no Sidecar.
