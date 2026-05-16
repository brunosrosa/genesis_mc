# coding=utf-8
import re

# ========================
# Fix SGR Synthesizer
# ========================
with open('src-tauri/src/cognition/sgr_synthesizer.rs', 'r', encoding='utf-8') as f:
    sgr_code = f.read()

# We need to make sure the SGR struct has no f64 mismatch in the test.
# wait, sgr_code was ALREADY patched by patch_sgr.py successfully earlier and NOT git checked out.
# So I just need to fix the f64/i32 error. The error is expected f64, found i32.
# Let's fix line 316 in sgr_synthesizer.rs
# I'll just restore sgr_synthesizer.rs to git state first, and re-patch it cleanly.
