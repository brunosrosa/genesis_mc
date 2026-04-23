import os
import glob
import numpy as np
from sentence_transformers import SentenceTransformer
from sklearn.metrics.pairwise import cosine_similarity
from sklearn.cluster import AgglomerativeClustering

RAW_DIR = "scratch/clean_sources"
OUTPUT_FILE = "scratch/combined_master_clean.md"
THRESHOLD = 0.95

print("Iniciando SODA Knowledge Curator (SemHash)...")

# 1. Carregar documentos
files = glob.glob(os.path.join(RAW_DIR, "*.md"))
docs = []
for f in files:
    with open(f, "r", encoding="utf-8") as file:
        content = file.read()
        docs.append({"path": f, "content": content, "length": len(content)})

if not docs:
    print("Nenhum documento encontrado em raw_sources.")
    exit()

print(f"[{len(docs)}] Documentos carregados. Carregando modelo SemHash (MiniLM)...")

# 2. Gerar embeddings
model = SentenceTransformer('all-MiniLM-L6-v2')
texts = [d["content"] for d in docs]
embeddings = model.encode(texts)

print(f"Calculando matriz de similaridade e aplicando poda de redundância (Threshold: {THRESHOLD})...")
# 3. Deduplicação
sim_matrix = cosine_similarity(embeddings)
duplicates = set()
for i in range(len(docs)):
    if i in duplicates:
        continue
    for j in range(i + 1, len(docs)):
        if sim_matrix[i][j] >= THRESHOLD:
            # Resolução por densidade (escolhe o documento com mais contexto/tamanho)
            if docs[i]["length"] >= docs[j]["length"]:
                duplicates.add(j)
            else:
                duplicates.add(i)

unique_docs = [docs[i] for i in range(len(docs)) if i not in duplicates]
print(f"Poda concluída: {len(docs) - len(unique_docs)} documentos ruídos removidos. {len(unique_docs)} fontes puras sobreviventes.")

# 4. Agrupamento Temático
print("Agrupando fontes sobreviventes em eixos temáticos...")
if len(unique_docs) > 1:
    unique_embeddings = [embeddings[i] for i in range(len(docs)) if i not in duplicates]
    # Distância = 1 - similaridade
    # Como cosine_similarity pode dar levemente > 1 por float error, usamos np.clip
    dist_matrix = np.clip(1 - cosine_similarity(unique_embeddings), 0, 1)
    
    # Threshold de 0.6 para criar "blocos de conhecimento" amplos
    clusterer = AgglomerativeClustering(n_clusters=None, metric='precomputed', linkage='average', distance_threshold=0.6)
    labels = clusterer.fit_predict(dist_matrix)
else:
    labels = [0]

clusters = {}
for doc, label in zip(unique_docs, labels):
    if label not in clusters:
        clusters[label] = []
    clusters[label].append(doc)

# 5. Consolidação (Doc Combiner)
print(f"Forjando matriz estruturada mestre em {OUTPUT_FILE}...")
with open(OUTPUT_FILE, "w", encoding="utf-8") as out:
    out.write("# 🧠 SODA CANONICAL KNOWLEDGE BASE\n")
    out.write(f"> Gerado pelo @soda-knowledge-curator.\n")
    out.write(f"> Fontes Originais: {len(docs)} | Fontes Puras: {len(unique_docs)} | Temas Identificados: {len(clusters)}\n\n")
    
    for cluster_id, docs_in_cluster in clusters.items():
        out.write(f"## 🧩 Eixo Temático {cluster_id + 1}\n\n")
        for doc in docs_in_cluster:
            out.write(doc["content"])
            out.write("\n\n---\n\n")

print("✨ Cura Mnemônica concluída com sucesso! O Context Bloat foi erradicado.")
