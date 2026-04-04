FROM ghcr.io/astral-sh/uv:bookworm-slim
ENV UV_COMPILE_BYTECODE=1
ENV UV_LINK_MODE=copy
RUN apt-get update && apt-get install -y libgl1 libglib2.0-0 && rm -rf /var/lib/apt/lists/*
RUN uv tool install docling-mcp
ENTRYPOINT ["docling-mcp-server"]