# ESPECIFICAÇÃO TÉCNICA E CADERNO DE TDD: MARCO 5.11.0 (TASKS 139 & 141)

## 🧠 1. Task 139 — Motor Real de `intent` (Logit Probing Epistêmico na CPU)

### 1.1 Racional de Hardware e Termodinâmica
Para sustentar a soberania local sob a restrição física de 6GB de VRAM da RTX 2060m, o SODA V6 adota a técnica de **Logit Probing** em vez da geração de texto autorregressiva tradicional. O objetivo do avaliador epistêmico é medir a ambiguidade e a intenção do usuário em tempo constante $\mathcal{O}(1)$ na primeira e única passagem de tensores (*prefill pass*), poupando VRAM e ciclos de GPU.

### 1.2 O Cálculo da Entropia de Shannon
Em vez de permitir que o modelo Gemma E2B (Tier 0.5) disserte ou justifique suas respostas, nós o enjaulamos por meio de um **Verbalizer** que projeta o estado oculto final sobre dois tokens de controle estritos: `"0"` (falso/baixo risco/sem ambiguidade) e `"1"` (verdadeiro/alto risco/ambíguo).

O motor em Rust (`LlamaCpp4LogitEngine`) realiza o forward pass na CPU Host via instruções vetoriais AVX2 (`n_gpu_layers = 0`). Ao final do processamento, recuperamos os logits brutos destes dois tokens por meio do método `llama_get_logits_ith` na FFI:

$$l_0 = \text{logit}(\text{"0"}), \quad l_1 = \text{logit}(\text{"1"})$$

Aplicamos a função **Softmax** de forma atômica para extrair a distribuição de probabilidade calibrada:

$$P(\text{"0"}) = \frac{e^{l_0}}{e^{l_0} + e^{l_1}}, \quad P(\text{"1"}) = \frac{e^{l_1}}{e^{l_0} + e^{l_1}}$$

A incerteza epistêmica do prompt de entrada (ambiguidade de intenção) é mensurada através do cálculo da **Entropia de Shannon (H)**:

$$H(X) = - \left( P(\text{"0"}) \log_2 P(\text{"0"}) + P(\text{"1"}) \log_2 P(\text{"1"}) \right)$$

* Se $H(X) \ge 0.75$ (alto grau de incerteza/entropia), o disjuntor de ambiguidade é violado, indicando que o prompt do usuário possui múltiplos caminhos lógicos contraditórios. O sistema paralisa e aciona a interrupção socrática.

---

## 🛡️ 2. Task 141 — O Canal de Interrupção Socrática CLI (HITL sem UI)

### 2.1 Racional de Pragmatismo e Isenção de UI
Como a Milestone 4 (Frontend Canvas em Svelte 5) está temporariamente inativa, o SODA V6 contorna o uso do "Agent Inbox" e do "Blast Radius Canvas" visuais. A governança humana (HITL) contra mutações cegas e corrupção silenciosa de dados (SDC) é transferida para o terminal de chat ativo.

### 2.2 O Ciclo de Bloqueio e Resgate (Tokio Stdin Capture)
A interrupção socrática é acionada sob duas condições estritas de falha ou incerteza:
1. **Incerteza Epistêmica:** O cálculo da Entropia de Shannon na CPU (Task 139) ultrapassa o limiar de $0.75$.
2. **Falha do Ralph Loop:** O ciclo autônomo de compilação local (Red-Green-Refactor) no Shadow Workspace tenta e falha consecutivamente por 3 vezes sem obter Exit Code 0.

#### O Rito de Interrupção:
1. O daemon em Rust suspende assincronamente a execução do loop de eventos principal do Tokio.
2. Recupera o `git diff` do Shadow Workspace por meio de chamadas nativas do `gitoxide`.
3. Imprime no terminal/chat ativo o diff das alterações pretendidas e o relatório de erros de compilação ou ambiguidade.
4. Formula uma **Pergunta Socrática de Duas Pernas** (sem usar o inquisitório "Por que", priorizando "Como", "O que" e "Para que" para evitar respostas defensivas).
5. Bloqueia a execução do Tokio consumindo a entrada padrão:
   ```rust
   let mut user_input = String::new();
   tokio::io::stdin().read_line(&mut user_input).await?;
   ```
6. O operador deve digitar explicitamente uma palavra-chave de homologação (`"approve"`, `"accept"`, `"yes"`) no console do chat para autorizar o rebase semântico na branch `main`. Caso o usuário rejeite (`"reject"`, `"no"`), o Shadow Workspace é sumariamente descartado (SIGKILL/delete) e o monorepo permanece intocado.

---

## 🚦 3. Caderno de Testes TDD (DoD GREEN)

Escreveremos e rodaremos os seguintes testes sob `cargo test --bin souls_mcp_server`:

1. `test_logit_probing_entropy_calculation`: Fornece logits cruzados controlados ao helper de entropia, asseverando que a Softmax e a equação de Shannon retornam o valor exato, violando o disjuntor quando acima de 0.75.
2. `test_socratic_cli_block_and_approval`: Mocks de buffers de `stdin` e `stdout` simulam uma interrupção socrática ativa de Ralph Loop falho, asseverando que a thread de processamento bloqueia, exibe o diff e apenas autoriza o merge após receber o token `"approve"`.
3. `test_gemma_cpu_isolation`: Valida que o empacotador de inferência instancia o `LlamaCpp4LogitEngine` com `n_gpu_layers = 0`, provando que 100% dos tensores rodam na CPU do Host via AVX2 com consumo zero de VRAM.
