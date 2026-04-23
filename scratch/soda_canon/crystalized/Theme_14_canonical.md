# 🧠 SODA CANONICAL MANUAL - EIXO 14
> Síntese Arquitetural: Vantage - Medição de Durable Skills
> Status: Cristalizado via Curador Arquitetural

## 🎯 Axioma Central: Medição Cognitiva em Escala
A avaliação de habilidades duráveis (colaboração, criatividade, pensamento crítico) no ecossistema SODA deve transcender a coleta passiva de dados. O protocolo **Vantage** é adaptado para o Genesis MC como um sistema de **Interação Dialética Local**, onde agentes sintéticos (AI Teammates) coabitam o hardware local para desafiar e elicitar evidências comportamentais do usuário.

---

## 🛠 Diretivas de Arquitetura (Genesis MC)

### 1. Inferência Dialética Soberana (Sovereign Dialectics)
*   **Restrição Estrita**: É terminantemente proibido o uso de APIs de nuvem para a simulação de agentes. O princípio de soberania exige que o "Executive LLM" (orquestrador da conversa) e os "AI Teammates" rodem localmente.
*   **Implementação**: Utilizar **Rust (Tokio)** para gerenciar instâncias concorrentes de modelos quantizados via **llama.cpp (mmap)**.
*   **Otimização de Hardware**: Para a **RTX 2060m**, os modelos devem ser carregados com pesos compartilhados ou quantização agressiva (4-bit) para suportar a carga tripla (Usuário + 2 Agentes) sem colapso de barramento.

### 2. Fluxo de Direcionamento (Executive Steering)
*   O "Executive LLM" atua como um mestre de cerimônias invisível, ajustando os prompts dos agentes sintéticos para forçar o usuário a demonstrar proficiência (ex: gerando conflitos controlados para medir resolução de problemas).
*   Toda a lógica de troca de contexto entre agentes deve ser feita via **Tauri IPC Zero-Copy**, mantendo a interface **Svelte 5** como uma casca passiva de renderização.

### 3. Poda de Tecnologias Tóxicas
*   **Eliminar**: Qualquer menção a frameworks de avaliação baseados em Python/Flask, Node.js ou Dashboards em React.
*   **Substituir**: O motor de "Auto-Rating" descrito no Vantage deve ser compilado como uma biblioteca estática ou rodar como um worker de baixa prioridade em Rust, evitando interferência na thread principal de UI.

---

## 🔍 Auditoria Crítica: Furos e Conflitos

O framework **Vantage** original, embora robusto psicometricamente, apresenta falhas graves para a filosofia SODA:
1.  **Dependência de Nuvem**: O artigo assume escalabilidade via infraestrutura Google Cloud. No SODA, a escalabilidade é **horizontal e local**, dependendo da eficiência do código Rust em aproveitar diretivas **AVX2**.
2.  **Privacidade de Dados**: O paper não detalha a proteção dos metadados comportamentais. No SODA, transcrições e scores psicométricos são encriptados em repouso e nunca saem do perímetro soberano do usuário.
3.  **Latência de Imersão**: O modelo de nuvem introduz latência que quebra a "validade ecológica". A execução local bare-metal garante respostas instantâneas, preservando a fluidez da interação social simulada.

---

## 🛡 Conclusão Técnica (O "Porquê")
O SODA integra o Vantage não como um serviço externo, mas como um subsistema de **Auto-Aprimoramento Cognitivo Offline**. A união de Rust e modelos locais quantizados permite que o Genesis MC transforme o hardware em um laboratório de desenvolvimento humano privado e de alta fidelidade.
