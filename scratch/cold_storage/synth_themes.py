import os
import time
from google import genai
from google.genai import types

# Carregar do .env caso não esteja injetado no terminal
env_path = os.path.join(os.path.dirname(__file__), "..", ".env")
if os.path.exists(env_path):
    with open(env_path, "r", encoding="utf-8") as f:
        for line in f:
            if line.startswith("GOOGLE_API_KEY="):
                os.environ["GEMINI_API_KEY"] = line.strip().split("=", 1)[1].strip('"\'')
            elif line.startswith("GEMINI_API_KEY="):
                os.environ["GEMINI_API_KEY"] = line.strip().split("=", 1)[1].strip('"\'')
            elif line.startswith("GOOGLE_MODEL_FAST="):
                os.environ["GOOGLE_MODEL_FAST"] = line.strip().split("=", 1)[1].strip('"\'')

# Verifica API Key
if "GEMINI_API_KEY" not in os.environ:
    raise ValueError("A variável de ambiente GEMINI_API_KEY ou GOOGLE_API_KEY não está definida!")

client = genai.Client()

SYSTEM_INSTRUCTION = """Você é o Curador Arquitetural do projeto SODA (Sovereign Operating Data Architecture - Genesis MC).
Sua missão é ler o aglomerado de textos extraídos da web a seguir e sintetizá-lo em um Manual Canônico.
Regras Inegociáveis:
1. ARQUITETURA PURA: A arquitetura SODA usa estritamente Rust (Tokio) no backend e Svelte 5 + Tauri v2 no frontend. O Frontend é uma interface PASSIVA. Toda lógica reside no Rust (IPC Zero-Copy).
2. PODA TÓXICA: ELIMINE SUMARIAMENTE e ignore parágrafos que exaltem tutoriais de React, Node.js daemons, Electron, VDOM ou arquiteturas Server-Side Rendering (Next.js).
3. HARDWARE AWARE: PRESERVE menções a otimizações bare-metal, limitações da iGPU (gargalos de barramento), e diretivas de execução AVX2 para a CPU ou llama.cpp mmap para a RTX 2060m.
4. AUDITORIA CRÍTICA (Furos e Conflitos): Analise ativamente o texto em busca de contradições, ideias conflitantes, "furos" ou "buracos" na estratégia do Genesis MC em relação ao material lido. Se a fonte propõe algo frágil para o SODA, evidencie a falha.
5. CONSOLIDAÇÃO DO "PORQUÊ": Não crie um resumo superficial. Consolide o 'PORQUÊ' técnico. Una conceitos soltos em uma explicação coesa.
"""

def process_themes():
    raw_dir = "soda_canon/raw"
    out_dir = "soda_canon/crystalized"
    
    os.makedirs(out_dir, exist_ok=True)
    
    themes = [f for f in os.listdir(raw_dir) if f.endswith(".md")]
    themes.sort()
    
    total = len(themes)
    print(f"Iniciando síntese sequencial de {total} temas com Proteção Anti-Quota.")
    
    for idx, theme_file in enumerate(themes, 1):
        raw_path = os.path.join(raw_dir, theme_file)
        out_path = os.path.join(out_dir, theme_file.replace(".md", "_canonical.md"))
        
        # Só pula se o arquivo existe E tem conteúdo razoável (evita arquivos vazios ou pela metade)
        if os.path.exists(out_path) and os.path.getsize(out_path) > 100:
            print(f"[{idx}/{total}] Pulando {theme_file} (já processado e validado).")
            continue
            
        print(f"[{idx}/{total}] Processando {theme_file}...", end=" ", flush=True)
        
        with open(raw_path, "r", encoding="utf-8") as f:
            full_content = f.read()
            
        # O modelo lite tem 1M tokens de janela. Podemos fatiar em partes de 800.000 chars.
        chunk_size = 800000
        chunks = [full_content[i:i+chunk_size] for i in range(0, len(full_content), chunk_size)]
        
        success_all = True
        
        # Limpa o arquivo de saída antes de fazer append
        open(out_path, "w", encoding="utf-8").close()
        
        for c_idx, chunk in enumerate(chunks):
            if len(chunks) > 1:
                print(f"\n      -> Processando Parte {c_idx+1}/{len(chunks)} do {theme_file}...", end=" ", flush=True)
                
            retries = 6
            backoff = 180
            success_chunk = False
            
            while retries > 0 and not success_chunk:
                try:
                    # Forçando gemini-2.5-flash-lite
                    target_model = "gemini-2.5-flash-lite"
                    
                    # Se for chunk > 1, ajusta a instrução para não repetir cabeçalhos
                    inst = SYSTEM_INSTRUCTION
                    if c_idx > 0:
                        inst += "\n\nAVISO: Esta é a PARTE " + str(c_idx+1) + " deste arquivo. Não crie uma nova introdução. Apenas adicione os NOVOS axiomas, regras e conceitos técnicos encontrados nesta parte. Vá direto ao ponto."
                        
                    response = client.models.generate_content(
                        model=target_model,
                        contents=chunk,
                        config=types.GenerateContentConfig(
                            system_instruction=inst,
                            temperature=0.2,
                        )
                    )
                    
                    if not response.text or len(response.text.strip()) < 50:
                        print("\n[ERRO] Resposta vazia. Retentando...")
                        retries -= 1
                        time.sleep(10)
                        continue

                    # Append no arquivo
                    with open(out_path, "a", encoding="utf-8") as out_f:
                        if c_idx > 0:
                            out_f.write("\n\n---\n\n")
                        out_f.write(response.text)
                    
                    print("OK.")
                    success_chunk = True
                    
                except Exception as e:
                    err_msg = str(e)
                    if "429" in err_msg or "quota" in err_msg.lower() or "retrydelay" in err_msg.lower():
                        print(f"\n[ALERTA] Rate Limit atingido! Aguardando {backoff}s para o balde resetar...")
                        time.sleep(backoff)
                        retries -= 1
                    else:
                        print(f"\n[ERRO INESPERADO]: {e}")
                        retries -= 1
                        time.sleep(15)
                        
            if not success_chunk:
                success_all = False
                break
                
            # Dorme entre chunks para não estourar o balde contínuo
            if c_idx < len(chunks) - 1:
                time.sleep(30)
                
        if not success_all:
            print(f"\n[AVISO] Não foi possível processar {theme_file}. Pulando para o próximo...")
            # Apaga o arquivo vazio gerado para que ele seja reprocessado numa próxima execução
            if os.path.exists(out_path) and os.path.getsize(out_path) == 0:
                os.remove(out_path)
            continue
            
        if idx < total:
            print("  Respiro de 30 segundos anti-ban para proteger o limite de Tokens/Minuto...")
            time.sleep(30)

if __name__ == "__main__":
    process_themes()
