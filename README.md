# Base64 Studio 🦀 (Rust Edition)

Software web e API REST de alta performance para codificação e decodificação Base64, reescrito em **Rust** com **Axum** e design moderno com **Glassmorphism**.

---

## ✨ Funcionalidades

- **Encode & Decode Instantâneo**: Modo interativo ("Live Mode") ou por botão com atalho de teclado (`Ctrl + Enter`).
- **Suporte a Variantes**:
  - Base64 padrão (RFC 4648).
  - Base64 URL-Safe (`-` e `_`).
  - Suporte completo a **Data URLs** (`data:<mime>;base64,<dados>`).
  - Correção automática de padding faltante (`=`).
- **Detecção Mágica de Binários**:
  - Reconhecimento automático de formatos (PDF, PNG, JPG, GIF, WebP, ZIP, BMP, EXE, ELF, MP3, WAV, OGG, GZ, 7z).
  - **Inspetor Hexadecimal**: Visualização de offsets, bytes hex e representação ASCII com formatação monoespaçada.
  - **Player e Prévia Multimídia**: Prévia visual direta para imagens e reprodutor de áudio embutido.
- **Suporte a Arquivos**:
  - Drag & Drop (arraste e solte diretamente na interface).
  - Download inteligente com extensão e MIME type inferidos.
- **Design Moderno & Responsivo**:
  - Paleta escura rica com efeitos de glassmorphism e iluminação néon.
  - Painéis lado a lado com estatísticas de caracteres e bytes em tempo real.
  - Botão de inversão rápida (`⇄ Inverter`) para testes de ida e volta.
  - Sistema de notificações flutuantes (toasts).
- **API REST Integrada**:
  - `POST /api/encode` (JSON, URL-encoded ou Multipart).
  - `POST /api/decode` (JSON, URL-encoded ou Multipart).
  - `GET /health` (Healthcheck).

---

## 🚀 Como Executar

### Pré-requisitos
- [Rust](https://rustup.rs/) (versão 1.80+) ou [Docker](https://www.docker.com/).

### Executando com Cargo
```bash
# Compilar e rodar
cargo run --release

# O servidor iniciará em http://0.0.0.0:8080
```

### Variáveis de Ambiente
| Variável | Padrão | Descrição |
| :--- | :--- | :--- |
| `PORT` | `8080` | Porta onde o servidor escuta |
| `MAX_MB` | `10` | Tamanho máximo permitido para upload (em MB) |

Exemplo customizado:
```bash
PORT=3000 MAX_MB=25 cargo run --release
```

---

## 🐳 Executando com Docker

Construa a imagem multi-stage e rode o container:

```bash
docker build -t rust-base64 .
docker run -p 8080:8080 -e MAX_MB=15 rust-base64
```

> **Compatibilidade OpenShift/Kubernetes**: A imagem foi construída para suportar execução com UID arbitrário de usuário (`chmod -R g=u /opt/app`).

---

## 📡 Documentação da API

### 1. Codificar (`POST /api/encode`)
```bash
# Via JSON
curl -X POST http://localhost:8080/api/encode \
  -H "Content-Type: application/json" \
  -d '{"data": "Olá Rust!", "urlsafe": false}'

# Via Multipart (Upload de arquivo)
curl -X POST http://localhost:8080/api/encode \
  -F "file=@foto.png" \
  -F "urlsafe=true"
```

### 2. Decodificar (`POST /api/decode`)
```bash
# Via JSON
curl -X POST http://localhost:8080/api/decode \
  -H "Content-Type: application/json" \
  -d '{"data": "T2zDoSBSdXN0IQ=="}'

# Via Data URL
curl -X POST http://localhost:8080/api/decode \
  -H "Content-Type: application/json" \
  -d '{"data": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="}'
```

### 3. Auto-Decode de Textos e YAML Secrets (`POST /api/auto-decode`)
Varre um texto ou manifesto YAML/JSON completo, descobre automaticamente tokens Base64 e os decodifica (em Secrets Kubernetes/OpenShift com `type: Opaque`, converte `data:` para `stringData:` com os valores decodificados):

```bash
# Via JSON (retorna JSON com texto decodificado + array de segredos isolados)
curl -X POST http://localhost:8080/api/auto-decode \
  -H "Content-Type: application/json" \
  -d '{"text": "connection_string: RGF0YSBTb3VyY2U9UENPUlBPUkFUSVZPLDMzNDE7..."}'

# Via Raw Text / Arquivo YAML (recebe o arquivo e devolve o YAML decodificado direto)
curl -X POST "http://localhost:8080/api/auto-decode?raw=true" \
  -H "Content-Type: text/plain" \
  --data-binary @secret-k8s.yaml
```

### 4. Diffchecker Comparador de Textos (`POST /api/diff`)
Compara dois textos e calcula adições, remoções e linhas inalteradas:

```bash
curl -X POST http://localhost:8080/api/diff \
  -H "Content-Type: application/json" \
  -d '{
    "original": "versão antiga",
    "modified": "versão nova com alterações",
    "ignore_case": false,
    "ignore_whitespace": false
  }'
```

---

## 🧪 Testes Unitários

Para rodar a suíte de testes de codificação, padding, detecção de magic bytes e hexdump:

```bash
cargo test
```
