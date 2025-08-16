# Imagem base enxuta
FROM python:3.12-slim

# Variáveis de ambiente úteis
ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PORT=8080 \
    PIP_NO_CACHE_DIR=1

# Dependências do sistema (opcional, mas bom ter ca-certificates)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/app

# Instala deps primeiro (cache mais eficiente)
COPY requirements.txt .
RUN pip install -r requirements.txt

# Copia a aplicação
COPY app.py .

# Ajusta permissões para rodar com UID arbitrário (padrão do OpenShift)
# Sem chown: garantimos que grupo tem as mesmas permissões do owner
RUN chmod -R g=u /opt/app

# OpenShift costuma injetar PORT, mas garantimos fallback para 8080
EXPOSE 8080

# Sobe com gunicorn (produção) apontando para objeto Flask "app" em app.py
CMD gunicorn -w 2 -k gthread --threads 8 --timeout 60 \
    -b 0.0.0.0:${PORT:-8080} app:app