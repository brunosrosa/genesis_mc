import os
import glob
import trafilatura
import sys

sys.stdout.reconfigure(encoding='utf-8')

RAW_DIR = "scratch/raw_sources"
CLEAN_DIR = "scratch/clean_sources"
os.makedirs(CLEAN_DIR, exist_ok=True)

files = glob.glob(os.path.join(RAW_DIR, "*.md"))
print(f"Iniciando Limpeza Léxica em {len(files)} arquivos...")

for i, f in enumerate(files):
    filename = os.path.basename(f)
    clean_path = os.path.join(CLEAN_DIR, filename)
    
    if os.path.exists(clean_path):
        print(f"[{i+1}/{len(files)}] {filename} já limpo. Pulando.")
        continue
        
    with open(f, "r", encoding="utf-8") as file:
        lines = file.readlines()
        
    # Extrair metadados para garantir integridade
    title = lines[0].strip() if len(lines) > 0 else "# Untitled"
    url_line = next((l for l in lines[:10] if l.startswith("Source URL:")), "")
    url = url_line.replace("Source URL:", "").strip()
    
    source_type_line = next((l for l in lines[:10] if l.startswith("Source Type:")), "")
    source_id_line = next((l for l in lines[:10] if l.startswith("Source ID:")), "")
    
    # Heurística: Uploads puros de "Deep Research" não têm URL externa (ou a URL é inválida/interna)
    if not url or url == "N/A" or not url.startswith("http"):
        print(f"[{i+1}/{len(files)}] Deep Research protegido. Preservando {filename[:15]}...")
        with open(clean_path, "w", encoding="utf-8") as out:
            out.writelines(lines)
        continue
        
    # Processo de Purificação via Trafilatura
    print(f"[{i+1}/{len(files)}] Extraindo ruído léxico da URL: {url}...")
    downloaded = trafilatura.fetch_url(url)
    
    if downloaded:
        # Extrai apenas texto, ignorando links vazios, menus, etc.
        cleaned_text = trafilatura.extract(downloaded)
        if cleaned_text:
            with open(clean_path, "w", encoding="utf-8") as out:
                out.write(f"{title}\n")
                out.write(url_line + "\n")
                out.write(source_type_line + "\n")
                out.write(source_id_line + "\n\n")
                out.write(cleaned_text)
            continue
            
    # Fallback agressivo se o Trafilatura falhar (ex: bloqueio por antibot)
    print(f"  -> Trafilatura bloqueado. Acionando Fallback Heurístico agressivo...")
    with open(clean_path, "w", encoding="utf-8") as out:
        start_idx = 0
        for idx, line in enumerate(lines):
            if line.startswith("Source ID:"):
                start_idx = idx + 1
                break
        
        out.writelines(lines[:start_idx]) # Preserva Header
        
        # Corta os primeiros 10% (menu) e últimos 20% (rodapé/sugestões de posts)
        content_lines = lines[start_idx:]
        if len(content_lines) > 100:
            top_cut = int(len(content_lines) * 0.1)
            bot_cut = int(len(content_lines) * 0.2)
            content_lines = content_lines[top_cut:-bot_cut]
            
        out.writelines(content_lines)

print("\n🚀 Limpeza Léxica finalizada com sucesso! Web Scraper Noise erradicado.")
