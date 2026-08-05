import os
import glob
from datetime import datetime

def compile_adrs():
    adrs_dir = r"Z:\souls_mc\docs\decisions\adrs"
    output_file = r"Z:\souls_mc\docs\observability\context_dumps\_ADRs_ALL.txt"
    
    # Ensure output directory exists
    os.makedirs(os.path.dirname(output_file), exist_ok=True)
    
    # Get all .md files in the adrs directory and sort them alphabetically
    md_pattern = os.path.join(adrs_dir, "*.md")
    md_files = glob.glob(md_pattern)
    md_files.sort(key=lambda x: os.path.basename(x).lower())
    
    # Generate timestamp
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    
    with open(output_file, "w", encoding="utf-8") as outfile:
        # Write timestamp as the first line
        outfile.write(f"_ADRs_ALL Gerado em: {timestamp}\n")
        
        for file_path in md_files:
            file_name = os.path.basename(file_path)
            abs_path = os.path.abspath(file_path)
            
            # Read content
            with open(file_path, "r", encoding="utf-8") as infile:
                content = infile.read()
                
            # Write header
            outfile.write(f"\n### ====================================================================================================\n")
            outfile.write(f"ARQUIVO: {file_name}\n")
            outfile.write(f"CAMINHO: {abs_path}\n")
            outfile.write(f"---\n")
            
            # Write content
            outfile.write(content)
            # Ensure there is a trailing newline
            if not content.endswith("\n"):
                outfile.write("\n")
                
    print(f"ADRs compiled successfully to {output_file}")

if __name__ == "__main__":
    compile_adrs()
