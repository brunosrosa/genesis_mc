# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "httpx",
#     "pydantic",
#     "pydantic-ai",
#     "python-dotenv",
#     "openai",
#     "google-auth",
#     "google-api-python-client",
#     "rich",
# ]
# ///

import asyncio
import os
import sys
from pathlib import Path
from dotenv import load_dotenv

# Add project root to sys.path so we can import etl
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

# Load the root .env file so we have GOOGLE_API_KEY, OPENAI_API_KEY, etc.
load_dotenv(Path(__file__).parent.parent.parent / ".env", override=True)

from etl.etl_orchestrator import main, _build_parser

if __name__ == "__main__":
    # Fix the UnicodeEncodeError by forcing utf-8 for stdout/stderr
    sys.stdout.reconfigure(encoding='utf-8')
    sys.stderr.reconfigure(encoding='utf-8')
    
    parser = _build_parser()
    args = parser.parse_args()
    
    sys.exit(asyncio.run(
        main(
            lote_id=args.lote_id,
            batch_size=args.batch_size,
            vault_db=args.vault_db,
            dry_run=args.dry_run,
            limit=args.limit,
        )
    ))
