/**
 * Live Typeset Preview & Syntax Checker using POST /api/parse and local KaTeX.
 */
import { parse } from './api.js';

let debounceTimer = null;

export function initPreview({ inputEl, formatEl, previewEl, varsContainer, onVariablesDetected }) {
  const update = () => {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      const expression = inputEl.value.trim();
      const input_format = formatEl.value;

      if (!expression) {
        previewEl.innerHTML = '<span class="text-dim text-xs">Enter an expression to see live typeset preview...</span>';
        if (varsContainer) varsContainer.innerHTML = '';
        return;
      }

      try {
        const res = await parse({ expression, input_format });
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
          previewEl.innerHTML = `<span class="text-danger text-xs">${res.error || 'Syntax error'}</span>`;
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
        previewEl.innerHTML = `<span class="text-danger text-xs">${err.message}</span>`;
      }
    }, 180);
  };

  inputEl.addEventListener('input', update);
  formatEl.addEventListener('change', update);
  return { refresh: update };
}
