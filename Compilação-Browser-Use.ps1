# 1. Cria o diretório e a senha VNC
New-Item -Path "C:\Users\rosas\Dev_Projects\soda-browser-sidecar" -ItemType Directory -Force
Set-Location -Path "C:\Users\rosas\Dev_Projects\soda-browser-sidecar"
Set-Content -Path "vnc_password.txt" -Value "soda2026"

# 2. Injeta o Dockerfile Imutável
$dockerfile = @'
FROM ghcr.io/astral-sh/uv:python3.11-bookworm-slim
ENV UV_COMPILE_BYTECODE=1
ENV UV_LINK_MODE=copy
ENV BROWSER_USE_TELEMETRY=false
ENV ANONYMIZED_TELEMETRY=false
RUN apt-get update && apt-get install -y xvfb x11vnc fluxbox novnc libglib2.0-0 libnss3 libatk1.0-0 libatk-bridge2.0-0 libcups2 libdrm2 libxkbcommon0 libxcomposite1 libxdamage1 libxrandr2 libgbm1 libasound2 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
RUN uv pip install --system browser-use[mcp] playwright
RUN playwright install --with-deps chromium
COPY vnc_password.txt /app/vnc_password.txt
RUN x11vnc -storepasswd $(cat /app/vnc_password.txt) /etc/x11vnc.pass
EXPOSE 5900
ENTRYPOINT ["/bin/sh", "-c", "Xvfb :99 -screen 0 1920x1080x24 & x11vnc -display :99 -forever -rfbauth /etc/x11vnc.pass -shared & tail -f /dev/null"]
'@
Set-Content -Path "Dockerfile" -Value $dockerfile

# 3. Injeta o Docker Compose
$compose = @'
version: '3.8'
services:
  browser-use-sidecar:
    build: .
    image: browser-use-local:secure
    container_name: browser-use-sidecar
    shm_size: '2gb'
    stdin_open: true
    tty: false
    cap_drop:
      - ALL
    cap_add:
      - SYS_ADMIN
    ports:
      - "127.0.0.1:5900:5900"
    environment:
      - DISPLAY=:99
      - BROWSER_HEADLESS=false
      - BROWSER_USE_TELEMETRY=false
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - OPENAI_BASE_URL=${OPENAI_BASE_URL:-https://openrouter.ai/api/v1}
      - OPENAI_MODEL_NAME=${OPENROUTER_DEFAULT_MODEL:-openrouter/free}
    restart: unless-stopped
'@
Set-Content -Path "docker-compose.yml" -Value $compose

# 4. Compila e sobe o contêiner, depois volta para o projeto
docker-compose up -d --build
Set-Location -Path "C:\Users\rosas\Dev_Projects\genesis_mc"