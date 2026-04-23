import os
import re

def split_themes():
    input_file = "combined_master_clean.md"
    output_dir = "soda_canon/raw"
    
    if not os.path.exists(output_dir):
        os.makedirs(output_dir, exist_ok=True)
        
    print(f"Lendo {input_file}...")
    with open(input_file, "r", encoding="utf-8") as f:
        content = f.read()
        
    # Usando regex para dividir nos eixos temáticos
    # O padrão procura por "## 🧩 Eixo Temático " seguido do número e qualquer coisa até o próximo Eixo.
    parts = re.split(r'(## 🧩 Eixo Temático \d+)', content)
    
    # parts[0] is everything before the first Eixo Temático (the header)
    header = parts[0]
    
    count = 0
    # O split retorna [header, "## 🧩 Eixo Temático 1", content1, "## 🧩 Eixo Temático 2", content2, ...]
    for i in range(1, len(parts), 2):
        theme_header = parts[i]
        theme_content = parts[i+1]
        
        # Extrair o número do tema para o nome do arquivo
        match = re.search(r'Eixo Temático (\d+)', theme_header)
        if match:
            theme_num = int(match.group(1))
            filename = f"Theme_{theme_num:02d}.md"
            filepath = os.path.join(output_dir, filename)
            
            with open(filepath, "w", encoding="utf-8") as out_f:
                out_f.write(header + "\n\n" + theme_header + theme_content)
                
            print(f"Salvo: {filepath}")
            count += 1
            
    print(f"Total de temas extraídos: {count}")

if __name__ == "__main__":
    split_themes()
