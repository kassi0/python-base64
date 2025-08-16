#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
Flask Base64 Tool — encode/decode de texto e arquivos
- UI web (/) mostra SEMPRE o resultado.
- Download é OPCIONAL (botão) — exceto no DECODE de arquivo (auto-download).
- API JSON /api/encode e /api/decode continuam disponíveis.
"""

import os
import io
import base64
import binascii
from datetime import datetime
from flask import (
    Flask, request, render_template_string,
    send_file, jsonify, abort
)

APP_TITLE = "Base64 Encoder/Decoder"
MAX_MB = int(os.getenv("MAX_MB", "10"))  # tamanho máximo de upload em MB
PORT = int(os.getenv("PORT", "8080"))
DEBUG = os.getenv("DEBUG", "0") == "1"

app = Flask(__name__)
app.config["MAX_CONTENT_LENGTH"] = MAX_MB * 1024 * 1024

# ---------- Helpers ----------
def _fix_padding(b: bytes) -> bytes:
    pad = (-len(b)) % 4
    return b + (b"=" * pad)

def b64_encode(data: bytes, urlsafe: bool = False) -> bytes:
    fn = base64.urlsafe_b64encode if urlsafe else base64.b64encode
    return fn(data)

def b64_decode(data_b64: bytes, urlsafe: bool = False) -> bytes:
    compact = b"".join(data_b64.split())
    compact = _fix_padding(compact)
    fn = base64.urlsafe_b64decode if urlsafe else base64.b64decode
    try:
        return fn(compact)
    except (binascii.Error, ValueError) as e:
        raise ValueError("Base64 inválido ou mal formatado.") from e

def guess_download_name(original: str | None, action: str) -> str:
    base = (original or "resultado").rsplit("/", 1)[-1]
    if action == "encode":
        return f"{base}.b64.txt"
    else:
        if ".b64." in base:
            return base.replace(".b64.", ".dec.")
        if base.endswith(".b64") or base.endswith(".txt"):
            return base.rsplit(".", 1)[0]
        return f"{base}.decoded"

def hexdump_preview(b: bytes, maxlen: int = 64) -> str:
    sl = b[:maxlen]
    hexstr = " ".join(f"{x:02x}" for x in sl)
    return hexstr + (" …" if len(b) > maxlen else "")

# ---------- UI ----------
INDEX_HTML = """
<!doctype html>
<html lang="pt-br">
<head>
  <meta charset="utf-8">
  <title>{{ title }}</title>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="icon" href="{{ url_for('static', filename='favicon.ico') }}" type="image/x-icon">
  <style>
    :root { --bg:#0f172a; --card:#111827; --muted:#9ca3af; --accent:#22c55e; --red:#ef4444; --txt:#e5e7eb; }
    * { box-sizing: border-box; }
    body { margin:0; font-family: system-ui, -apple-system, Segoe UI, Roboto, Ubuntu, "Helvetica Neue", Arial;
           background: radial-gradient(1200px 600px at 70% -10%, #1f2937 0%, #0b1022 60%, #000 100%),
                       linear-gradient(#0b1022, #000);
           color: var(--txt); }
    .wrap { max-width: 980px; margin: 40px auto; padding: 16px; }
    .card { background: rgba(17,24,39,.75); border: 1px solid #1f2937; border-radius: 16px; padding: 20px; backdrop-filter: blur(6px); }
    h1 { margin: 0 0 8px; font-size: 20px; font-weight: 600; letter-spacing: .3px; }
    .sub { color: var(--muted); font-size: 13px; margin-bottom: 16px; }
    .row { display:flex; gap:16px; flex-wrap: wrap; }
    .col { flex:1 1 320px; min-width: 280px; }
    textarea, input[type="text"] { width: 100%; background: #0b1022; color: var(--txt); border:1px solid #1f2937; border-radius: 10px; padding:10px; font: inherit; }
    textarea { min-height: 180px; resize: vertical; }
    .file { padding:8px; border:1px dashed #334155; border-radius:10px; }
    .muted { color: var(--muted); font-size: 12px; }
    .toggle { display:flex; align-items:center; gap:8px; }
    .btns { display:flex; gap:8px; flex-wrap: wrap; }
    button { background:#111827; border:1px solid #334155; color: var(--txt); padding:10px 14px; border-radius:10px; cursor:pointer; }
    button.primary { background: #166534; border-color:#166534; }
    button.danger { background: #7f1d1d; border-color:#7f1d1d; }
    .ok { color: var(--accent); }
    .err { color: var(--red); }
    .footer { margin-top:10px; display:flex; justify-content:space-between; gap:8px; align-items:center; }
    code.inline { background:#0b1022; padding:2px 6px; border-radius:6px; border:1px solid #1f2937; }
    .result-block { margin-top:20px; }
    .preview { background:#0b1022; border:1px solid #1f2937; border-radius:10px; padding:10px; font-family:ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace; font-size:12px; color:#cbd5e1; overflow:auto; }
    form.inline { display:inline; margin-left:auto; }
  </style>
</head>
<body>
  <div class="wrap">
    <div class="card">
      <h1>🔐 {{ title }}</h1>
      <div class="sub">Encode/Decode Base64 — resultado aparece aqui; download é opcional (exceto decode de arquivo).</div>

      <form class="row" action="/process" method="post" enctype="multipart/form-data">
        <div class="col">
          <h3>Entrada</h3>
          <textarea name="input_text" placeholder="Cole seu texto ou Base64 aqui (opcional se enviar arquivo)">{{ input_text or "" }}</textarea>
          <div class="file" style="margin-top:10px">
            <label>Arquivo (opcional): <input type="file" name="input_file" /></label>
            <div class="muted">Se enviar arquivo: no ENCODE o Base64 aparece abaixo; no DECODE o arquivo é baixado automaticamente.</div>
          </div>
        </div>

        <div class="col">
          <h3>Ação</h3>
          <div style="display:flex; gap:10px; margin-bottom:10px;">
            <label><input type="radio" name="action" value="encode" {{ 'checked' if action!='decode' else '' }}> Encode</label>
            <label><input type="radio" name="action" value="decode" {{ 'checked' if action=='decode' else '' }}> Decode</label>
          </div>

          <div class="toggle" style="margin-bottom:10px;">
            <input id="urlsafe" type="checkbox" name="urlsafe" {{ 'checked' if urlsafe else '' }}/>
            <label for="urlsafe">Usar URL-safe (<code class="inline">-</code> e <code class="inline">_</code> no lugar de <code class="inline">+</code> e <code class="inline">/</code>)</label>
          </div>

          <div class="btns">
            <button class="primary" type="submit">Processar</button>
            <button class="danger" type="button" id="btnClear">Limpar</button>
          </div>

          {% if message %}
            <p class="{{ 'ok' if success else 'err' }}" style="margin-top:12px">{{ message }}</p>
          {% endif %}
        </div>
      </form>
        <script>
        (() => {
        const btn = document.getElementById('btnClear');
        if (!btn) return;

        btn.addEventListener('click', () => {
            // campos de entrada
            const ta   = document.querySelector('textarea[name="input_text"]');
            const file = document.querySelector('input[type="file"][name="input_file"]');
            if (ta)   ta.value = '';
            if (file) file.value = '';   // alguns browsers não limpam com reset

            // resultado de texto
            const res = document.getElementById('result');
            if (res) res.value = '';

            // remove blocos de resultado/mensagens renderizados pelo servidor
            document.querySelectorAll('.result-block').forEach(el => el.remove());
            const msg = document.querySelector('p.ok, p.err');
            if (msg) msg.remove();
        });
        })();
        </script>
      {% if result_text is not none %}
        <div class="result-block">
          <h3>Resultado (texto)</h3>
          <textarea readonly id="result">{{ result_text }}</textarea>
          <div class="footer">
            <span class="muted">Tamanho: {{ (result_text|length) }} chars.</span>
            <div style="display:flex; gap:8px; margin-left:auto;">
              <button onclick="copyRes()">Copiar</button>
              {% if downloadable_text %}
              <form class="inline" action="/download_text" method="post">
                <input type="hidden" name="filename" value="{{ download_text_name }}">
                <input type="hidden" name="content" value="{{ result_text }}">
                <button type="submit">Baixar arquivo (.txt)</button>
              </form>
              {% endif %}
            </div>
          </div>
        </div>
        <script>
          function copyRes(){
            const ta = document.getElementById('result');
            ta.select(); ta.setSelectionRange(0, 999999);
            document.execCommand('copy');
          }
        </script>
      {% endif %}

      {% if result_is_binary %}
        <div class="result-block">
          <h3>Resultado (binário)</h3>
          <div class="preview">Prévia (hex): {{ preview_hex }}</div>
          <div class="footer">
            <span class="muted">Conteúdo não-textual detectado.</span>
            <form class="inline" action="/download_binary" method="post">
              <input type="hidden" name="filename" value="{{ download_binary_name }}">
              <input type="hidden" name="content_b64" value="{{ result_binary_b64 }}">
              <button type="submit">Baixar arquivo</button>
            </form>
          </div>
        </div>
      {% endif %}

      <div style="margin-top:24px" class="muted">
        API: <code class="inline">POST /api/encode</code>, <code class="inline">POST /api/decode</code>
        — JSON: <code class="inline">{ "data": "texto", "urlsafe": true }</code>
        ou multipart com <code class="inline">file</code>.
      </div>
      <div class="muted" style="margin-top:8px">
        Limite de upload: {{ max_mb }} MB — configure com <code class="inline">MAX_MB</code>.
      </div>
    </div>
  </div>
</body>
</html>
"""

@app.route("/", methods=["GET"])
def index():
    return render_template_string(
        INDEX_HTML,
        title=APP_TITLE,
        input_text="",
        result_text=None,
        result_is_binary=False,
        preview_hex="",
        action="encode",
        urlsafe=False,
        message=None,
        success=True,
        max_mb=MAX_MB,
        downloadable_text=False,
        download_text_name="resultado.txt",
        download_binary_name="resultado.bin",
        result_binary_b64=""
    )

@app.route('/favicon.ico')
def favicon():
    return redirect(url_for('static', filename='favicon.ico'), code=302)

@app.route("/process", methods=["POST"])
def process():
    action = request.form.get("action", "encode")
    urlsafe = bool(request.form.get("urlsafe"))
    input_text = (request.form.get("input_text") or "").strip()
    file = request.files.get("input_file")
    has_file = file and file.filename

    if action not in ("encode", "decode"):
        return abort(400, "Ação inválida.")

    # Caso: ARQUIVO enviado
    if has_file:
        filename = file.filename
        data = file.read()

        if action == "encode":
            # Mostra na tela e deixa download opcional
            out_b64 = b64_encode(data, urlsafe=urlsafe).decode("ascii")
            return render_template_string(
                INDEX_HTML, title=APP_TITLE, input_text=input_text,
                result_text=out_b64, result_is_binary=False, preview_hex="",
                action=action, urlsafe=urlsafe,
                message="Encode OK. Você pode baixar como arquivo se quiser.",
                success=True, max_mb=MAX_MB,
                downloadable_text=True,
                download_text_name=guess_download_name(filename, "encode"),
                download_binary_name="",
                result_binary_b64=""
            )
        else:
            # DECODE de ARQUIVO => auto-download (binário)
            try:
                out = b64_decode(data, urlsafe=urlsafe)
            except ValueError as e:
                return render_template_string(
                    INDEX_HTML, title=APP_TITLE, input_text=input_text,
                    result_text=None, result_is_binary=False, preview_hex="",
                    action=action, urlsafe=urlsafe, message=str(e),
                    success=False, max_mb=MAX_MB,
                    downloadable_text=False, download_text_name="",
                    download_binary_name="", result_binary_b64=""
                )
            out_io = io.BytesIO(out)
            out_name = guess_download_name(filename, "decode")
            return send_file(out_io, as_attachment=True, download_name=out_name, mimetype="application/octet-stream")

    # Caso: TEXTO (sem arquivo)
    if not input_text:
        return render_template_string(
            INDEX_HTML, title=APP_TITLE, input_text="",
            result_text=None, result_is_binary=False, preview_hex="",
            action=action, urlsafe=urlsafe,
            message="Forneça texto ou selecione um arquivo.", success=False, max_mb=MAX_MB,
            downloadable_text=False, download_text_name="",
            download_binary_name="", result_binary_b64=""
        )

    if action == "encode":
        out = b64_encode(input_text.encode("utf-8"), urlsafe=urlsafe).decode("ascii")
        return render_template_string(
            INDEX_HTML, title=APP_TITLE, input_text=input_text,
            result_text=out, result_is_binary=False, preview_hex="",
            action=action, urlsafe=urlsafe, message="Encode OK.", success=True, max_mb=MAX_MB,
            downloadable_text=True, download_text_name=guess_download_name("texto", "encode"),
            download_binary_name="", result_binary_b64=""
        )
    else:
        try:
            raw = b64_decode(input_text.encode("ascii", "ignore"), urlsafe=urlsafe)
        except ValueError as e:
            return render_template_string(
                INDEX_HTML, title=APP_TITLE, input_text=input_text,
                result_text=None, result_is_binary=False, preview_hex="",
                action=action, urlsafe=urlsafe, message=str(e), success=False, max_mb=MAX_MB,
                downloadable_text=False, download_text_name="",
                download_binary_name="", result_binary_b64=""
            )
        # Se for texto UTF-8, mostra; se não, exibe prévia hex e botão de download binário (opcional)
        try:
            out_text = raw.decode("utf-8")
            return render_template_string(
                INDEX_HTML, title=APP_TITLE, input_text=input_text,
                result_text=out_text, result_is_binary=False, preview_hex="",
                action=action, urlsafe=urlsafe, message="Decode OK (texto).", success=True, max_mb=MAX_MB,
                downloadable_text=True, download_text_name=guess_download_name("texto_decodificado", "decode"),
                download_binary_name="", result_binary_b64=""
            )
        except UnicodeDecodeError:
            return render_template_string(
                INDEX_HTML, title=APP_TITLE, input_text=input_text,
                result_text=None, result_is_binary=True, preview_hex=hexdump_preview(raw),
                action=action, urlsafe=urlsafe, message="Decode OK (binário detectado). Use o botão para baixar.", success=True, max_mb=MAX_MB,
                downloadable_text=False, download_text_name="",
                download_binary_name=guess_download_name("binario", "decode"),
                result_binary_b64=base64.b64encode(raw).decode("ascii")
            )

# ---------- Downloads opcionais ----------
@app.post("/download_text")
def download_text():
    filename = (request.form.get("filename") or "resultado.txt").strip() or "resultado.txt"
    content = request.form.get("content", "")
    buf = io.BytesIO(content.encode("utf-8"))
    return send_file(buf, as_attachment=True, download_name=filename, mimetype="text/plain; charset=utf-8")

@app.post("/download_binary")
def download_binary():
    filename = (request.form.get("filename") or "resultado.bin").strip() or "resultado.bin"
    content_b64 = request.form.get("content_b64", "")
    try:
        raw = base64.b64decode(_fix_padding(content_b64.encode("ascii", "ignore")))
    except (binascii.Error, ValueError):
        return abort(400, "Conteúdo Base64 inválido para download.")
    buf = io.BytesIO(raw)
    return send_file(buf, as_attachment=True, download_name=filename, mimetype="application/octet-stream")

# ---------- API ----------
@app.post("/api/encode")
def api_encode():
    urlsafe = request.args.get("urlsafe", "").lower() in ("1", "true", "yes") or bool(request.form.get("urlsafe")) or (request.json or {}).get("urlsafe", False)
    f = request.files.get("file")
    if f and f.filename:
        data = f.read()
        b64 = b64_encode(data, urlsafe=urlsafe).decode("ascii")
        return jsonify({"ok": True, "mode": "file", "base64": b64, "urlsafe": urlsafe})
    payload = request.get_json(silent=True) or {}
    data_text = payload.get("data") or request.form.get("data")
    if not data_text:
        return jsonify({"ok": False, "error": "Faltou 'data' (texto) ou 'file'."}), 400
    b64 = b64_encode(data_text.encode("utf-8"), urlsafe=urlsafe).decode("ascii")
    return jsonify({"ok": True, "mode": "text", "base64": b64, "urlsafe": urlsafe})

@app.post("/api/decode")
def api_decode():
    urlsafe = request.args.get("urlsafe", "").lower() in ("1", "true", "yes") or bool(request.form.get("urlsafe")) or (request.json or {}).get("urlsafe", False)
    f = request.files.get("file")
    if f and f.filename:
        data_b64 = f.read()
        try:
            raw = b64_decode(data_b64, urlsafe=urlsafe)
        except ValueError as e:
            return jsonify({"ok": False, "error": str(e)}), 400
        return jsonify({"ok": True, "mode": "file", "decoded_base64": base64.b64encode(raw).decode("ascii"), "urlsafe": urlsafe})
    payload = request.get_json(silent=True) or {}
    data_b64_text = payload.get("data") or request.form.get("data")
    if not data_b64_text:
        return jsonify({"ok": False, "error": "Faltou 'data' (Base64) ou 'file'."}), 400
    try:
        raw = b64_decode(data_b64_text.encode("ascii", "ignore"), urlsafe=urlsafe)
    except ValueError as e:
        return jsonify({"ok": False, "error": str(e)}), 400
    try:
        text = raw.decode("utf-8")
        return jsonify({"ok": True, "mode": "text", "text": text, "urlsafe": urlsafe})
    except UnicodeDecodeError:
        return jsonify({
            "ok": True,
            "mode": "binary",
            "note": "Conteúdo não era UTF-8; devolvido como Base64 para segurança.",
            "decoded_base64": base64.b64encode(raw).decode("ascii"),
            "urlsafe": urlsafe
        })

# ---------- Erros simpáticos ----------
@app.errorhandler(413)
def too_large(_e):
    return jsonify({"ok": False, "error": f"Arquivo maior que {MAX_MB} MB."}), 413

@app.errorhandler(400)
def bad_request(e):
    return jsonify({"ok": False, "error": str(e)}), 400

# ---------- Main ----------
if __name__ == "__main__":
    print(f"[{datetime.now().isoformat(timespec='seconds')}] {APP_TITLE} rodando em 0.0.0.0:{PORT} (limite {MAX_MB} MB)")
    app.run(host="0.0.0.0", port=PORT, debug=DEBUG)
