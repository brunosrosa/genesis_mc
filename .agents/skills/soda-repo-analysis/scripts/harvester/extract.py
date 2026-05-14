import json
import subprocess
import os
import sys

sys.stdout.reconfigure(encoding='utf-8')

NOTEBOOK_ID = "89c46d12-14de-4970-8085-77f9f2d34e4f"
RAW_DIR = "scratch/raw_sources"
os.makedirs(RAW_DIR, exist_ok=True)

print(f"Fetching sources for notebook {NOTEBOOK_ID}...")
result = subprocess.run("nlm list sources " + NOTEBOOK_ID + " --json", shell=True, capture_output=True, text=True, encoding="utf-8")
if result.returncode != 0:
    print("Error listing sources:", result.stderr)
    exit(1)

sources = json.loads(result.stdout)
print(f"Found {len(sources)} sources. Starting extraction...")

for i, src in enumerate(sources):
    src_id = src["id"]
    src_title = src.get("title", "Untitled").replace("/", "-").replace("\\", "-")
    file_path = os.path.join(RAW_DIR, f"{src_id}.md")
    
    if os.path.exists(file_path):
        print(f"[{i+1}/{len(sources)}] Skipping {src_id} (already downloaded)")
        continue
        
    print(f"[{i+1}/{len(sources)}] Extracting {src_id} ({src_title[:30]})...")
    
    content_res = subprocess.run("nlm content source " + src_id, shell=True, capture_output=True, text=True, encoding="utf-8")
    if content_res.returncode == 0:
        with open(file_path, "w", encoding="utf-8") as f:
            f.write(f"# {src.get('title', 'Untitled')}\n")
            f.write(f"Source URL: {src.get('url', 'N/A')}\n")
            f.write(f"Source Type: {src.get('type', 'N/A')}\n")
            f.write(f"Source ID: {src_id}\n\n")
            f.write(content_res.stdout)
    else:
        print(f"  -> Failed to extract {src_id}: {content_res.stderr.strip()}")

print("Extraction complete.")
