#!/usr/bin/env python3
r"""
SOULS MC (SODA V6) — Conversion Script: GLiClass Multilang Ultra to ONNX.

This script converts the HuggingFace safetensors weights for knowledgator/gliclass-multilang-ultra
into an optimized ONNX Runtime graph (`gliclass_multilang.onnx`) and saves `tokenizer.json`.

Source model path: C:/Users/rosas/.lmstudio/models/knowledgator/gliclass-multilang-ultra
Target directory:  src-tauri/models/ or configured model path
"""

import os
import sys
import json
from pathlib import Path

DEFAULT_MODEL_DIR = r"C:\Users\rosas\.lmstudio\models\knowledgator\gliclass-multilang-ultra"
OUTPUT_DIR = Path(__file__).parent.parent / "models"

import torch
import torch.nn as nn

class GLiClassONNXWrapper(nn.Module):
    def __init__(self, model, class_token_index, example_token_index):
        super().__init__()
        self.model = model
        self.class_token_index = class_token_index
        self.example_token_index = example_token_index

    def forward(self, input_ids, attention_mask):
        out = self.model(
            input_ids=input_ids,
            attention_mask=attention_mask,
            class_token_index=self.class_token_index,
            example_token_index=self.example_token_index,
        )
        if hasattr(out, "logits"):
            return out.logits
        return out[0]

def main():
    model_dir = os.environ.get("GLICLASS_MODEL_DIR", DEFAULT_MODEL_DIR)
    out_dir = Path(os.environ.get("GLICLASS_OUTPUT_DIR", str(OUTPUT_DIR)))
    out_dir.mkdir(parents=True, exist_ok=True)

    print(f"[SOULS GLiClass Converter] Reading from: {model_dir}")
    print(f"[SOULS GLiClass Converter] Output directory: {out_dir}")

    onnx_path = out_dir / "gliclass_multilang.onnx"

    try:
        from gliclass import GLiClassModel
        from gliclass.data_processing import GLiClassDataset
        from gliclass.data_processing.collator import GLiClassDataCollator
        from torch.utils.data import DataLoader
        from transformers import AutoTokenizer

        print("[SOULS GLiClass Converter] Loading tokenizer & GLiClassModel...")
        tokenizer = AutoTokenizer.from_pretrained(model_dir)
        tokenizer.save_pretrained(str(out_dir))

        model = GLiClassModel.from_pretrained(model_dir)
        model.eval()

        triage_labels = ["unsafe_prompt", "valid_intent"]

        print("[SOULS GLiClass Converter] Preparing zero-shot dataset inputs for ONNX trace...")
        dataset = GLiClassDataset(
            texts=["Test prompt for triage classification"],
            labels=triage_labels,
            tokenizer=tokenizer,
            model_config=model.config,
        )
        loader = DataLoader(dataset, batch_size=1, collate_fn=GLiClassDataCollator(tokenizer))
        batch = next(iter(loader))

        wrapper = GLiClassONNXWrapper(
            model,
            batch["class_token_index"],
            batch["example_token_index"],
        )
        wrapper.eval()

        print(f"[SOULS GLiClass Converter] Exporting ONNX graph to {onnx_path}...")
        torch.onnx.export(
            wrapper,
            (batch["input_ids"], batch["attention_mask"]),
            str(onnx_path),
            input_names=["input_ids", "attention_mask"],
            output_names=["logits"],
            dynamic_axes={
                "input_ids": {0: "batch_size", 1: "sequence_length"},
                "attention_mask": {0: "batch_size", 1: "sequence_length"},
                "logits": {0: "batch_size"},
            },
            opset_version=14,
            dynamo=False,
        )
        
        # Remove manifest fallback marker on clean ONNX export
        manifest_path = out_dir / "gliclass_manifest.json"
        if manifest_path.exists():
            manifest_path.unlink()

        print(f"[SOULS GLiClass Converter] SUCCESS: ONNX model exported cleanly to {onnx_path}!")

    except Exception as e:
        print(f"[SOULS GLiClass Converter] Exception during conversion: {e}")
        import traceback
        traceback.print_exc()
        print("[SOULS GLiClass Converter] Fallback: Generating manifest marker for factory dev fallback.")
        manifest = {
            "model_name": "gliclass-multilang-ultra",
            "source_path": model_dir,
            "status": "fallback_active",
            "error": str(e),
        }
        with open(out_dir / "gliclass_manifest.json", "w", encoding="utf-8") as f:
            json.dump(manifest, f, indent=2)
        print(f"[SOULS GLiClass Converter] Wrote manifest to {out_dir / 'gliclass_manifest.json'}")

if __name__ == "__main__":
    main()
