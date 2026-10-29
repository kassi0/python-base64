// Base64 Studio - Modern Rust Edition Client Logic

(function () {
  'use strict';

  // DOM Elements
  const inputText = document.getElementById('inputText');
  const resultText = document.getElementById('resultText');
  const fileInput = document.getElementById('fileInput');
  const btnProcess = document.getElementById('btnProcess');
  const btnSwap = document.getElementById('btnSwap');
  const btnClear = document.getElementById('btnClear');
  const btnCopy = document.getElementById('btnCopy');
  const btnDownload = document.getElementById('btnDownload');
  const modeEncodeBtn = document.getElementById('modeEncode');
  const modeDecodeBtn = document.getElementById('modeDecode');
  const modeAutoBtn = document.getElementById('modeAuto');
  const switchUrlSafe = document.getElementById('switchUrlSafe');
  const switchLive = document.getElementById('switchLive');
  const inputStats = document.getElementById('inputStats');
  const outputStats = document.getElementById('outputStats');
  const fileInfoBadge = document.getElementById('fileInfoBadge');
  const inputPane = document.getElementById('inputPane');
  const hexInspector = document.getElementById('hexInspector');
  const autoItemsContainer = document.getElementById('autoItemsContainer');
  const mediaPreviewBox = document.getElementById('mediaPreviewBox');
  const mediaPreviewImg = document.getElementById('mediaPreviewImg');
  const mediaPreviewAudio = document.getElementById('mediaPreviewAudio');
  const detectedBadge = document.getElementById('detectedBadge');
  const syncBadge = document.getElementById('syncBadge');
  const tabText = document.getElementById('tabText');
  const tabHex = document.getElementById('tabHex');
  const apiDrawer = document.getElementById('apiDrawer');
  const apiHeader = document.getElementById('apiHeader');
  const workspaceGrid = document.getElementById('workspaceGrid');
  const btnToggleLayout = document.getElementById('btnToggleLayout');
  const layoutIcon = document.getElementById('layoutIcon');
  const layoutText = document.getElementById('layoutText');
  const btnReencode = document.getElementById('btnReencode');

  // State
  let currentAction = 'encode'; // 'encode' | 'decode' | 'auto'
  let currentFile = null;
  let currentBinaryBytes = null;
  let currentDownloadFilename = 'resultado.txt';
  let currentMimeType = 'text/plain';
  let isBinaryMode = false;
  let isVerticalLayout = false;
  let liveDebounceTimer = null;
  let reverseLiveTimer = null;

  // Samples
  const samples = {
    text: "Olá! Bem-vindo ao Base64 Studio em Rust 🦀 — Rápido, seguro e moderno.",
    dataUrl: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
    jwt: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkFuZGVsIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
    k8sSecret: `apiVersion: v1
kind: Secret
metadata:
  name: db-renach-credentials
  namespace: prod-apps
type: Opaque
data:
  connection_string: RGF0YSBTb3VyY2U9UENPUlBPUkFUSVZPLDMzNDE7SW5pdGlhbCBDYXRhbG9nPWRiX3JlbmFjaDtDb25uZWN0UmV0cnlDb3VudD0xMDA7UGVyc2lzdCBTZWN1cml0eSBJbmZvPVRydWU7VXNlciBJRD11c3JfYXBwX3BvcnRhbF9zaXN0ZW1hcztQYXNzd29yZD14aWlUOGZjTEZueW95OUxyQ3lTTTtNdWx0aXBsZUFjdGl2ZVJlc3VsdFNldHM9dHJ1ZTtUcnVzdFNlcnZlckNlcnRpZmljYXRlPVRydWU7
  db_user: dXNyX2FwcF9wb3J0YWxfc2lzdGVtYXM=`
  };

  // Toast System
  function showToast(message, type = 'success') {
    const container = document.getElementById('toastContainer');
    if (!container) return;

    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    const icon = type === 'success' ? '✓' : '⚠️';
    toast.innerHTML = `<span style="font-weight:700;">${icon}</span><span>${message}</span>`;
    container.appendChild(toast);

    setTimeout(() => {
      toast.style.transition = 'opacity 0.3s ease, transform 0.3s ease';
      toast.style.opacity = '0';
      toast.style.transform = 'translateY(10px)';
      setTimeout(() => toast.remove(), 300);
    }, 2800);
  }

  // Formatting helpers
  function formatBytes(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function updateStats() {
    const inVal = inputText.value;
    const inBytes = new TextEncoder().encode(inVal).length;
    inputStats.textContent = `${inVal.length} chars • ${formatBytes(inBytes)}`;

    const outVal = resultText.value;
    if (isBinaryMode && currentBinaryBytes) {
      outputStats.textContent = `Binário • ${formatBytes(currentBinaryBytes.length)}`;
    } else {
      const outBytes = new TextEncoder().encode(outVal).length;
      outputStats.textContent = `${outVal.length} chars • ${formatBytes(outBytes)}`;
    }
  }

  // Hex dump generation client-side
  function generateHexDump(bytes, maxBytes = 512) {
    const len = Math.min(bytes.length, maxBytes);
    let out = '';
    for (let i = 0; i < len; i += 16) {
      const chunk = bytes.slice(i, i + 16);
      const offset = i.toString(16).padStart(8, '0');
      let hexPart = '';
      let asciiPart = '';

      for (let j = 0; j < 16; j++) {
        if (j < chunk.length) {
          const b = chunk[j];
          hexPart += b.toString(16).padStart(2, '0') + ' ';
          if (j === 7) hexPart += ' ';
          asciiPart += (b >= 32 && b <= 126) ? String.fromCharCode(b) : '.';
        } else {
          hexPart += '   ';
          if (j === 7) hexPart += ' ';
        }
      }
      out += `${offset}  ${hexPart} |${asciiPart}|\n`;
    }
    if (bytes.length > maxBytes) {
      out += `... [mostrando primeiros ${maxBytes} de ${bytes.length} bytes]\n`;
    }
    return out;
  }

  // Detect file magic signature
  function detectMagic(bytes) {
    if (bytes.length >= 5 && bytes[0] === 0x25 && bytes[1] === 0x50 && bytes[2] === 0x44 && bytes[3] === 0x46 && bytes[4] === 0x2D) {
      return { ext: '.pdf', mime: 'application/pdf', label: 'PDF Document', isImg: false, isAudio: false };
    }
    if (bytes.length >= 8 && bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4E && bytes[3] === 0x47) {
      return { ext: '.png', mime: 'image/png', label: 'PNG Image', isImg: true, isAudio: false };
    }
    if (bytes.length >= 3 && bytes[0] === 0xFF && bytes[1] === 0xD8 && bytes[2] === 0xFF) {
      return { ext: '.jpg', mime: 'image/jpeg', label: 'JPEG Image', isImg: true, isAudio: false };
    }
    if (bytes.length >= 6 && bytes[0] === 0x47 && bytes[1] === 0x49 && bytes[2] === 0x46) {
      return { ext: '.gif', mime: 'image/gif', label: 'GIF Image', isImg: true, isAudio: false };
    }
    if (bytes.length >= 12 && String.fromCharCode(...bytes.slice(0, 4)) === 'RIFF' && String.fromCharCode(...bytes.slice(8, 12)) === 'WEBP') {
      return { ext: '.webp', mime: 'image/webp', label: 'WebP Image', isImg: true, isAudio: false };
    }
    if (bytes.length >= 4 && bytes[0] === 0x50 && bytes[1] === 0x4B && bytes[2] === 0x03 && bytes[3] === 0x04) {
      return { ext: '.zip', mime: 'application/zip', label: 'ZIP Archive', isImg: false, isAudio: false };
    }
    if (bytes.length >= 2 && bytes[0] === 0x42 && bytes[1] === 0x4D) {
      return { ext: '.bmp', mime: 'image/bmp', label: 'BMP Image', isImg: true, isAudio: false };
    }
    if (bytes.length >= 3 && (String.fromCharCode(...bytes.slice(0, 3)) === 'ID3' || (bytes[0] === 0xFF && (bytes[1] & 0xFE) === 0xFA))) {
      return { ext: '.mp3', mime: 'audio/mpeg', label: 'MP3 Audio', isImg: false, isAudio: true };
    }
    if (bytes.length >= 12 && String.fromCharCode(...bytes.slice(0, 4)) === 'RIFF' && String.fromCharCode(...bytes.slice(8, 12)) === 'WAVE') {
      return { ext: '.wav', mime: 'audio/wav', label: 'WAV Audio', isImg: false, isAudio: true };
    }
    if (bytes.length >= 2 && bytes[0] === 0x1F && bytes[1] === 0x8B) {
      return { ext: '.gz', mime: 'application/gzip', label: 'GZIP Archive', isImg: false, isAudio: false };
    }
    return { ext: '.bin', mime: 'application/octet-stream', label: 'Binário Genérico', isImg: false, isAudio: false };
  }

  // Switch between text and hex tabs
  function showTab(type) {
    if (type === 'hex') {
      tabHex.classList.add('active');
      tabText.classList.remove('active');
      hexInspector.style.display = 'block';
      resultText.style.display = 'none';
    } else {
      tabText.classList.add('active');
      tabHex.classList.remove('active');
      hexInspector.style.display = 'none';
      resultText.style.display = 'block';
    }
  }

  // Reset binary state
  function resetBinaryState() {
    isBinaryMode = false;
    currentBinaryBytes = null;
    hexInspector.textContent = '';
    hexInspector.style.display = 'none';
    resultText.style.display = 'block';
    tabText.classList.add('active');
    tabHex.classList.remove('active');
    tabHex.style.display = 'none';
    mediaPreviewBox.style.display = 'none';
    mediaPreviewImg.style.display = 'none';
    mediaPreviewAudio.style.display = 'none';
    detectedBadge.style.display = 'none';
    if (autoItemsContainer) {
      autoItemsContainer.style.display = 'none';
      autoItemsContainer.innerHTML = '';
    }
  }

  function renderAutoItems(items) {
    if (!autoItemsContainer || !items || items.length === 0) return;
    autoItemsContainer.style.display = 'flex';
    autoItemsContainer.innerHTML = '';

    const header = document.createElement('div');
    header.style.cssText = 'font-size:12px; font-weight:600; color:var(--text-secondary); margin-bottom:2px; display:flex; justify-content:space-between; align-items:center;';
    header.innerHTML = `<span>🔍 Segredos Base64 Identificados (${items.length}):</span>`;
    autoItemsContainer.appendChild(header);

    items.forEach(item => {
      const card = document.createElement('div');
      card.className = 'secret-item-card';
      const keyLabel = item.key ? item.key : 'token';
      const preview = item.decoded.length > 60 ? item.decoded.substring(0, 60) + '...' : item.decoded;

      card.innerHTML = `
        <div class="secret-meta">
          <span class="secret-key-badge">${keyLabel}</span>
          <span class="secret-val-preview" title="${item.decoded.replace(/"/g, '&quot;')}">${preview}</span>
        </div>
        <button type="button" class="secret-copy-btn">Copiar</button>
      `;

      card.querySelector('.secret-copy-btn').addEventListener('click', () => {
        navigator.clipboard.writeText(item.decoded).then(() => {
          showToast(`Valor de "${keyLabel}" copiado!`);
        });
      });

      autoItemsContainer.appendChild(card);
    });
  }

  // Process Logic via Server API for 100% Rust-backed consistency
  async function processData() {
    const isUrlSafe = switchUrlSafe.checked;

    // Case 1: Uploading a file
    if (currentFile) {
      const formData = new FormData();
      formData.append('file', currentFile);
      if (isUrlSafe) formData.append('urlsafe', 'true');

      btnProcess.disabled = true;
      btnProcess.textContent = 'Processando...';

      try {
        let endpoint = '/api/encode';
        if (currentAction === 'decode') endpoint = '/api/decode';
        if (currentAction === 'auto') endpoint = '/api/auto-decode';

        const res = await fetch(endpoint, {
          method: 'POST',
          body: formData
        });

        const data = await res.json();
        if (!data.ok) throw new Error(data.error || 'Erro no processamento');

        if (currentAction === 'encode') {
          resetBinaryState();
          resultText.value = data.base64;
          currentDownloadFilename = `${currentFile.name}.b64.txt`;
          currentMimeType = 'text/plain';
          showToast('Arquivo codificado com sucesso!');
        } else if (currentAction === 'auto') {
          resetBinaryState();
          resultText.value = data.decoded_text;
          currentDownloadFilename = `${currentFile.name}.decoded`;
          currentMimeType = 'text/plain';
          if (data.items && data.items.length > 0) {
            renderAutoItems(data.items);
          }
          showToast(`${data.count} segredo(s) decodificado(s)!`);
        } else {
          // File decode
          handleDecodedPayload(data, currentFile.name);
        }
      } catch (err) {
        showToast(err.message, 'error');
      } finally {
        btnProcess.disabled = false;
        btnProcess.textContent = 'Processar';
        updateStats();
      }
      return;
    }

    // Case 2: Text processing
    const textVal = inputText.value.trim();
    if (!textVal) {
      showToast('Digite um texto ou selecione um arquivo.', 'error');
      return;
    }

    btnProcess.disabled = true;
    btnProcess.textContent = 'Processando...';

    try {
      if (currentAction === 'auto') {
        const res = await fetch('/api/auto-decode', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            text: textVal,
            convert_k8s: true
          })
        });

        const data = await res.json();
        if (!data.ok) throw new Error(data.error || 'Erro no processamento automático');

        resetBinaryState();
        resultText.value = data.decoded_text;
        currentDownloadFilename = data.is_k8s_secret ? 'secret_decodificado.yaml' : 'texto_decodificado.txt';
        currentMimeType = 'text/plain';

        if (data.items && data.items.length > 0) {
          renderAutoItems(data.items);
          showToast(`${data.count} segredo(s) Base64 decodificado(s) no texto!`);
        } else {
          showToast('Nenhum Base64 detectado no texto.', 'error');
        }
      } else {
        const endpoint = currentAction === 'encode' ? '/api/encode' : '/api/decode';
        const res = await fetch(endpoint, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            data: textVal,
            urlsafe: isUrlSafe
          })
        });

        const data = await res.json();
        if (!data.ok) throw new Error(data.error || 'Erro no processamento');

        if (currentAction === 'encode') {
          resetBinaryState();
          resultText.value = data.base64;
          currentDownloadFilename = 'encode_resultado.b64.txt';
          currentMimeType = 'text/plain';
          showToast('Texto codificado!');
        } else {
          handleDecodedPayload(data, 'arquivo');
        }
      }
    } catch (err) {
      showToast(err.message, 'error');
    } finally {
      btnProcess.disabled = false;
      btnProcess.textContent = 'Processar';
      updateStats();
    }
  }

  function handleDecodedPayload(data, baseName) {
    if (data.mode === 'text' && typeof data.text === 'string') {
      resetBinaryState();
      resultText.value = data.text;
      currentDownloadFilename = `${baseName}.txt`;
      currentMimeType = 'text/plain';
      showToast('Decodificado com sucesso (UTF-8)!');
    } else if (data.decoded_base64) {
      // Binary content
      isBinaryMode = true;
      const b64 = data.decoded_base64;
      const binStr = atob(b64);
      const len = binStr.length;
      const bytes = new Uint8Array(len);
      for (let i = 0; i < len; i++) {
        bytes[i] = binStr.charCodeAt(i);
      }
      currentBinaryBytes = bytes;

      const detected = detectMagic(bytes);
      currentDownloadFilename = `${baseName}${detected.ext}`;
      currentMimeType = detected.mime;

      detectedBadge.textContent = detected.label;
      detectedBadge.style.display = 'inline-flex';

      // Setup Hex
      tabHex.style.display = 'inline-block';
      hexInspector.textContent = generateHexDump(bytes, 512);

      // Check media preview
      if (detected.isImg) {
        mediaPreviewImg.src = `data:${detected.mime};base64,${b64}`;
        mediaPreviewImg.style.display = 'block';
        mediaPreviewAudio.style.display = 'none';
        mediaPreviewBox.style.display = 'flex';
      } else if (detected.isAudio) {
        mediaPreviewAudio.src = `data:${detected.mime};base64,${b64}`;
        mediaPreviewAudio.style.display = 'block';
        mediaPreviewImg.style.display = 'none';
        mediaPreviewBox.style.display = 'flex';
      } else {
        mediaPreviewBox.style.display = 'none';
      }

      showTab('hex');
      showToast(`Binário detectado: ${detected.label}!`);
    }
  }

  // Trigger live processing
  function triggerLive() {
    if (!switchLive.checked || currentFile) return;
    clearTimeout(liveDebounceTimer);
    liveDebounceTimer = setTimeout(() => {
      if (inputText.value.trim().length > 0) {
        processData();
      }
    }, 200);
  }

  function setLayout(vertical) {
    isVerticalLayout = vertical;
    if (vertical) {
      workspaceGrid.classList.add('layout-vertical');
      if (layoutIcon) layoutIcon.textContent = '⬌';
      if (layoutText) layoutText.textContent = 'Lado a Lado';
    } else {
      workspaceGrid.classList.remove('layout-vertical');
      if (layoutIcon) layoutIcon.textContent = '⬍';
      if (layoutText) layoutText.textContent = 'Vertical';
    }
  }

  if (btnToggleLayout) {
    btnToggleLayout.addEventListener('click', () => {
      setLayout(!isVerticalLayout);
    });
  }

  // Event Listeners
  modeEncodeBtn.addEventListener('click', () => {
    currentAction = 'encode';
    modeEncodeBtn.classList.add('active');
    modeDecodeBtn.classList.remove('active');
    if (modeAutoBtn) modeAutoBtn.classList.remove('active');
    inputText.placeholder = 'Cole seu texto original aqui (ou envie um arquivo)...';
    resultText.setAttribute('readonly', 'true');
    resultText.placeholder = 'O resultado aparecerá aqui...';
    if (syncBadge) syncBadge.style.display = 'none';
    if (btnReencode) btnReencode.style.display = 'none';
    resetBinaryState();
    triggerLive();
  });

  modeDecodeBtn.addEventListener('click', () => {
    currentAction = 'decode';
    modeDecodeBtn.classList.add('active');
    modeEncodeBtn.classList.remove('active');
    if (modeAutoBtn) modeAutoBtn.classList.remove('active');
    inputText.placeholder = 'Cole sua string Base64 ou Data URL (data:...;base64,)...';
    resultText.setAttribute('readonly', 'true');
    resultText.placeholder = 'O resultado aparecerá aqui...';
    if (syncBadge) syncBadge.style.display = 'none';
    if (btnReencode) btnReencode.style.display = 'none';
    resetBinaryState();
    triggerLive();
  });

  if (modeAutoBtn) {
    modeAutoBtn.addEventListener('click', () => {
      currentAction = 'auto';
      modeAutoBtn.classList.add('active');
      modeEncodeBtn.classList.remove('active');
      modeDecodeBtn.classList.remove('active');
      inputText.placeholder = 'Cole seu YAML de Secret (type: Opaque), JSON ou qualquer texto contendo Base64...';
      // Alterna automaticamente para modo vertical (embaixo) para visualização ampla
      setLayout(true);
      resultText.removeAttribute('readonly');
      resultText.placeholder = 'Resultado decodificado (Edite aqui para re-encodar automaticamente para Base64 no Modo Live)...';
      if (syncBadge) syncBadge.style.display = 'inline-flex';
      if (btnReencode) btnReencode.style.display = 'inline-flex';
      resetBinaryState();
      triggerLive();
    });
  }

  // Live Reverse Re-Encode: when editing decoded text in auto mode, update Base64
  resultText.addEventListener('input', () => {
    updateStats();
    if (currentAction === 'auto') {
      if (!switchLive.checked) return;
      clearTimeout(reverseLiveTimer);
      reverseLiveTimer = setTimeout(async () => {
        const val = resultText.value.trim();
        if (!val) return;
        try {
          const res = await fetch('/api/auto-encode', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ text: val })
          });
          const data = await res.json();
          if (data.ok) {
            inputText.value = data.encoded_text;
            updateStats();
            showToast('Re-encodado para Base64 no Modo Live!', 'success');
          }
        } catch (e) {
          console.error(e);
        }
      }, 350);
    }
  });

  if (btnReencode) {
    btnReencode.addEventListener('click', async () => {
      const val = resultText.value.trim();
      if (!val) {
        showToast('Nada no resultado para re-encodar.', 'error');
        return;
      }
      btnReencode.disabled = true;
      btnReencode.textContent = 'Re-encodando...';
      try {
        const res = await fetch('/api/auto-encode', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ text: val })
        });
        const data = await res.json();
        if (!data.ok) throw new Error(data.error || 'Erro ao re-encodar');
        inputText.value = data.encoded_text;
        updateStats();
        showToast('Secret re-encodado com sucesso para Base64 (data:)!');
      } catch (err) {
        showToast(err.message, 'error');
      } finally {
        btnReencode.disabled = false;
        btnReencode.innerHTML = '<span>🔒</span> Re-encodar Secret (data:)';
      }
    });
  }

  switchUrlSafe.addEventListener('change', triggerLive);

  inputText.addEventListener('input', () => {
    updateStats();
    triggerLive();
  });

  btnProcess.addEventListener('click', processData);

  // Keyboard shortcut Ctrl+Enter
  window.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      processData();
    }
  });

  // Swap input and output
  btnSwap.addEventListener('click', () => {
    const outVal = resultText.value;
    if (!outVal) return;
    inputText.value = outVal;
    resultText.value = '';
    currentFile = null;
    fileInfoBadge.style.display = 'none';
    resetBinaryState();

    // Toggle mode
    if (currentAction === 'encode') {
      modeDecodeBtn.click();
    } else {
      modeEncodeBtn.click();
    }
    showToast('Entrada e saída invertidas!');
  });

  // Clear button
  btnClear.addEventListener('click', () => {
    inputText.value = '';
    resultText.value = '';
    currentFile = null;
    fileInput.value = '';
    fileInfoBadge.style.display = 'none';
    resetBinaryState();
    updateStats();
    showToast('Área limpa.');
  });

  // Copy output
  btnCopy.addEventListener('click', () => {
    let copyContent = resultText.value;
    if (isBinaryMode && currentBinaryBytes) {
      // Copy base64 of binary
      const binStr = Array.from(currentBinaryBytes).map(b => String.fromCharCode(b)).join('');
      copyContent = btoa(binStr);
    }
    if (!copyContent) {
      showToast('Nada para copiar!', 'error');
      return;
    }
    navigator.clipboard.writeText(copyContent).then(() => {
      showToast('Copiado para a área de transferência!');
    }).catch(() => {
      showToast('Falha ao copiar.', 'error');
    });
  });

  // Download Output
  btnDownload.addEventListener('click', () => {
    if (isBinaryMode && currentBinaryBytes) {
      const blob = new Blob([currentBinaryBytes], { type: currentMimeType });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = currentDownloadFilename;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
      showToast(`Download de "${currentDownloadFilename}" iniciado.`);
    } else {
      const text = resultText.value;
      if (!text) {
        showToast('Nenhum resultado para baixar.', 'error');
        return;
      }
      const blob = new Blob([text], { type: 'text/plain;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = currentDownloadFilename || 'resultado.txt';
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
      showToast('Download do arquivo iniciado.');
    }
  });

  // File Upload Handlers
  fileInput.addEventListener('change', (e) => {
    if (e.target.files && e.target.files[0]) {
      handleFileSelected(e.target.files[0]);
    }
  });

  function handleFileSelected(file) {
    currentFile = file;
    fileInfoBadge.textContent = `📎 ${file.name} (${formatBytes(file.size)})`;
    fileInfoBadge.style.display = 'inline-flex';
    inputText.value = `[Arquivo selecionado: ${file.name} — clique em Processar]`;
    updateStats();
    showToast(`Arquivo "${file.name}" carregado.`);
  }

  // Drag and Drop
  ['dragenter', 'dragover'].forEach(name => {
    inputPane.addEventListener(name, (e) => {
      e.preventDefault();
      inputPane.classList.add('drag-over');
    });
  });

  ['dragleave', 'drop'].forEach(name => {
    inputPane.addEventListener(name, (e) => {
      e.preventDefault();
      inputPane.classList.remove('drag-over');
    });
  });

  inputPane.addEventListener('drop', (e) => {
    if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      handleFileSelected(e.dataTransfer.files[0]);
    }
  });

  // Tabs
  tabText.addEventListener('click', () => showTab('text'));
  tabHex.addEventListener('click', () => showTab('hex'));

  // Samples
  document.querySelectorAll('.sample-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const type = btn.getAttribute('data-sample');
      if (samples[type]) {
        currentFile = null;
        fileInput.value = '';
        fileInfoBadge.style.display = 'none';
        inputText.value = samples[type];
        if (type === 'k8sSecret') {
          if (modeAutoBtn) modeAutoBtn.click();
        } else if (type === 'dataUrl' || type === 'jwt') {
          modeDecodeBtn.click();
        } else {
          modeEncodeBtn.click();
        }
        updateStats();
        processData();
      }
    });
  });

  // API Drawer Accordion
  apiHeader.addEventListener('click', () => {
    apiDrawer.classList.toggle('open');
  });

  // ==========================================
  // TOP NAVIGATION SWITCHER (Base64 vs Diff)
  // ==========================================
  const navTabBase64 = document.getElementById('navTabBase64');
  const navTabDiff = document.getElementById('navTabDiff');
  const base64Section = document.getElementById('base64Section');
  const diffSection = document.getElementById('diffSection');
  const brandLogo = document.getElementById('brandLogo');
  const brandTitle = document.getElementById('brandTitle');
  const brandSubtitle = document.getElementById('brandSubtitle');

  function switchToTab(tab) {
    if (tab === 'diff') {
      navTabDiff.classList.add('active');
      navTabBase64.classList.remove('active');
      base64Section.style.display = 'none';
      diffSection.style.display = 'block';
      if (brandLogo) brandLogo.textContent = '⚖️';
      if (brandTitle) brandTitle.innerHTML = 'Diff<span class="highlight">checker</span> <span class="tag-rust">Comparador</span>';
      if (brandSubtitle) brandSubtitle.textContent = 'Comparação visual de textos, scripts e configurações • Destaque de alterações linha a linha';
      runDiff();
    } else {
      navTabBase64.classList.add('active');
      navTabDiff.classList.remove('active');
      base64Section.style.display = 'block';
      diffSection.style.display = 'none';
      if (brandLogo) brandLogo.textContent = '🦀';
      if (brandTitle) brandTitle.innerHTML = 'Base64 <span class="highlight">Studio</span> <span class="tag-rust">Rust 1.99</span>';
      if (brandSubtitle) brandSubtitle.textContent = 'Encode & Decode de alta performance • Zero dependências externas • Inspeção Hex & Magic Bytes';
    }
  }

  if (navTabBase64 && navTabDiff) {
    navTabBase64.addEventListener('click', () => {
      switchToTab('base64');
      window.location.hash = '';
    });
    navTabDiff.addEventListener('click', () => {
      switchToTab('diff');
      window.location.hash = 'diff';
    });
  }

  // Check URL hash on load
  if (window.location.hash === '#diff' || window.location.pathname === '/diff') {
    switchToTab('diff');
  }

  // ==========================================
  // DIFFCHECKER ENGINE & CONTROLS
  // ==========================================
  const diffTextA = document.getElementById('diffTextA');
  const diffTextB = document.getElementById('diffTextB');
  const diffStatsA = document.getElementById('diffStatsA');
  const diffStatsB = document.getElementById('diffStatsB');
  const btnRunDiff = document.getElementById('btnRunDiff');
  const btnSwapDiff = document.getElementById('btnSwapDiff');
  const switchDiffLive = document.getElementById('switchDiffLive');
  const switchDiffWhitespace = document.getElementById('switchDiffWhitespace');
  const switchDiffCase = document.getElementById('switchDiffCase');
  const btnDiffSample = document.getElementById('btnDiffSample');
  const btnDiffClear = document.getElementById('btnDiffClear');
  const diffBadgeAdd = document.getElementById('diffBadgeAdd');
  const diffBadgeDel = document.getElementById('diffBadgeDel');
  const diffBadgeUnchanged = document.getElementById('diffBadgeUnchanged');
  const diffContent = document.getElementById('diffContent');

  let diffDebounceTimer = null;

  function escapeHtml(str) {
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }

  function updateDiffStats() {
    if (!diffTextA || !diffTextB) return;
    const linesA = diffTextA.value ? diffTextA.value.split('\n').length : 0;
    const linesB = diffTextB.value ? diffTextB.value.split('\n').length : 0;
    diffStatsA.textContent = `${linesA} linhas • ${diffTextA.value.length} chars`;
    diffStatsB.textContent = `${linesB} linhas • ${diffTextB.value.length} chars`;
  }

  async function runDiff() {
    if (!diffTextA || !diffTextB) return;
    const orig = diffTextA.value;
    const mod = diffTextB.value;

    if (!orig && !mod) {
      diffBadgeAdd.textContent = '+0 Adições';
      diffBadgeDel.textContent = '-0 Remoções';
      diffBadgeUnchanged.textContent = '0 Inalteradas';
      diffContent.innerHTML = '<div style="padding: 32px; text-align: center; color: var(--text-muted); font-size: 13px;">Digite ou cole textos nos dois campos acima para visualizar as alterações linha a linha em tempo real.</div>';
      return;
    }

    try {
      const res = await fetch('/api/diff', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          original: orig,
          modified: mod,
          ignore_case: switchDiffCase ? switchDiffCase.checked : false,
          ignore_whitespace: switchDiffWhitespace ? switchDiffWhitespace.checked : false
        })
      });

      const data = await res.json();
      if (!data.ok) throw new Error(data.error || 'Erro ao comparar textos');

      const stats = data.stats;
      diffBadgeAdd.textContent = `+${stats.additions} Adições`;
      diffBadgeDel.textContent = `-${stats.deletions} Remoções`;
      diffBadgeUnchanged.textContent = `${stats.unchanged} Inalteradas`;

      if (stats.additions === 0 && stats.deletions === 0) {
        diffContent.innerHTML = `
          <div style="padding: 32px; text-align: center; color: #34d399; font-size: 13px;">
            ✓ Os textos são 100% idênticos! Nenhuma diferença encontrada.
          </div>
        `;
        return;
      }

      let html = '';
      data.lines.forEach(line => {
        const isAdded = line.kind === 'Added';
        const isRemoved = line.kind === 'Removed';
        const rowClass = isAdded ? 'diff-row added' : isRemoved ? 'diff-row removed' : 'diff-row';
        const gutterA = (line.original_line_num !== null && line.original_line_num !== undefined) ? line.original_line_num : '';
        const gutterB = (line.modified_line_num !== null && line.modified_line_num !== undefined) ? line.modified_line_num : '';
        const marker = isAdded ? '+' : isRemoved ? '-' : ' ';

        html += `
          <div class="${rowClass}">
            <div class="diff-gutter ${isRemoved ? 'removed' : ''}">${gutterA}</div>
            <div class="diff-gutter ${isAdded ? 'added' : ''}">${gutterB}</div>
            <div class="diff-marker">${marker}</div>
            <div class="diff-content">${escapeHtml(line.content)}</div>
          </div>
        `;
      });

      diffContent.innerHTML = html;
    } catch (err) {
      showToast(err.message, 'error');
    }
  }

  function triggerLiveDiff() {
    updateDiffStats();
    if (!switchDiffLive || !switchDiffLive.checked) return;
    clearTimeout(diffDebounceTimer);
    diffDebounceTimer = setTimeout(runDiff, 200);
  }

  if (diffTextA && diffTextB) {
    diffTextA.addEventListener('input', triggerLiveDiff);
    diffTextB.addEventListener('input', triggerLiveDiff);

    if (btnRunDiff) btnRunDiff.addEventListener('click', runDiff);

    if (btnSwapDiff) {
      btnSwapDiff.addEventListener('click', () => {
        const temp = diffTextA.value;
        diffTextA.value = diffTextB.value;
        diffTextB.value = temp;
        updateDiffStats();
        runDiff();
        showToast('Textos invertidos!');
      });
    }

    if (switchDiffWhitespace) switchDiffWhitespace.addEventListener('change', runDiff);
    if (switchDiffCase) switchDiffCase.addEventListener('change', runDiff);

    if (btnDiffClear) {
      btnDiffClear.addEventListener('click', () => {
        diffTextA.value = '';
        diffTextB.value = '';
        updateDiffStats();
        runDiff();
        showToast('Textos limpos.');
      });
    }

    if (btnDiffSample) {
      btnDiffSample.addEventListener('click', () => {
        diffTextA.value = `apiVersion: apps/v1
kind: Deployment
metadata:
  name: portal-sistemas
  replicas: 2
spec:
  template:
    spec:
      containers:
      - name: app
        image: registry.empresa.com/app:v1.2.0
        env:
        - name: DB_HOST
          value: pcorporativo.db.local
        - name: DB_PORT
          value: "3341"
        - name: DEBUG
          value: "false"`;

        diffTextB.value = `apiVersion: apps/v1
kind: Deployment
metadata:
  name: portal-sistemas
  replicas: 4
spec:
  template:
    spec:
      containers:
      - name: app
        image: registry.empresa.com/app:v1.3.1
        env:
        - name: DB_HOST
          value: pcorporativo-cluster.db.local
        - name: DB_PORT
          value: "3341"
        - name: DB_SSL
          value: "true"
        - name: DEBUG
          value: "false"`;

        updateDiffStats();
        runDiff();
        showToast('Exemplo carregado!');
      });
    }
  }

  // Initialize
  updateStats();
  updateDiffStats();
})();

