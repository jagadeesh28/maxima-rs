/**
 * Live Typeset Preview & Syntax Checker using POST /api/parse and local KaTeX.
 */
import { parse } from './api.js';

let debounceTimer = null;
let activeController = null;
let requestSeq = 0;

function showPreviewError(previewEl, message) {
  const span = document.createElement('span');
  span.className = 'text-danger text-xs';
  span.textContent = message;
  previewEl.replaceChildren(span);
}

export function initPreview({ inputEl, formatEl, previewEl, varsContainer, onVariablesDetected }) {
  const update = () => {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      const expression = inputEl.value.trim();
      const input_format = formatEl.value;

      if (activeController) {
        activeController.abort();
        activeController = null;
      }

      if (!expression) {
        previewEl.innerHTML = '<span class="text-dim text-xs">Enter an expression to see live typeset preview...</span>';
        if (varsContainer) varsContainer.innerHTML = '';
        return;
      }

      const seq = ++requestSeq;
      activeController = new AbortController();

      try {
        const res = await parse({ expression, input_format }, activeController.signal);
        if (seq !== requestSeq) return;

        if (res.ok && res.latex) {
          if (window.katex) {
            previewEl.innerHTML = window.katex.renderToString(res.latex, {
              displayMode: true,
              throwOnError: false
            });
          } else {
            previewEl.textContent = res.latex;
          }
        } else {
          showPreviewError(previewEl, res.error || 'Syntax error');
        }

        if (varsContainer) {
          varsContainer.innerHTML = '';
          if (res.variables && res.variables.length > 0) {
            res.variables.forEach(v => {
              const chip = document.createElement('span');
              chip.className = 'chip';
              chip.textContent = v;
              chip.title = `Variable: ${v}`;
              varsContainer.appendChild(chip);
            });
          }
        }

        if (onVariablesDetected && res.variables) {
          onVariablesDetected(res.variables);
        }
      } catch (err) {
        if (err && err.name === 'AbortError') return;
        if (seq !== requestSeq) return;
        showPreviewError(previewEl, err.message || 'Parse failed');
      }
    }, 180);
  };

  inputEl.addEventListener('input', update);
  formatEl.addEventListener('change', update);
  return { refresh: update };
}
