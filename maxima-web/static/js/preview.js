/**
 * Live Typeset Preview, Syntax Hint Bar, LaTeX Auto-Detect & Fix-It Caret Renderer.
 */
import { parse } from './api.js';
import { looksLikeLatex, buildErrorDiagnosticEl } from './fixit.js';

let debounceTimer = null;
let activeController = null;
let requestSeq = 0;

export function initPreview({
  inputEl,
  formatEl,
  previewEl,
  varsContainer,
  syntaxHintEl,
  latexBannerEl,
  switchLatexBtn,
  getActiveOperation,
  onVariablesDetected
}) {
  const updateHintAndBanner = () => {
    const fmt = formatEl ? formatEl.value : 'maxima';
    if (syntaxHintEl) {
      if (fmt === 'latex') {
        syntaxHintEl.innerHTML = 'LaTeX math mode: use <code>\\frac{a}{b}</code>, <code>\\sin(x)</code>, <code>\\pi</code>, <code>x^{2}</code>, <code>\\int x \\, dx</code>.';
      } else {
        syntaxHintEl.innerHTML = 'Maxima syntax: use <code>*</code> for multiply (<code>2*x</code>), <code>^</code> for powers (<code>x^2</code>), <code>%pi</code>, <code>%e</code>, <code>%i</code>, <code>%</code> for last result.';
      }
    }
    if (latexBannerEl) {
      const show = looksLikeLatex(inputEl.value, fmt);
      latexBannerEl.style.display = show ? 'flex' : 'none';
    }
  };

  if (switchLatexBtn && formatEl) {
    switchLatexBtn.addEventListener('click', () => {
      formatEl.value = 'latex';
      formatEl.dispatchEvent(new Event('change'));
      inputEl.focus();
    });
  }

  const update = () => {
    updateHintAndBanner();
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
          const op = typeof getActiveOperation === 'function' ? getActiveOperation() : '';
          const diag = buildErrorDiagnosticEl({
            expr: expression,
            message: res.error || 'Syntax error',
            span: res.error_span,
            operation: op,
            onApplyFix: (fixed) => {
              inputEl.value = fixed;
              inputEl.dispatchEvent(new Event('input'));
              inputEl.focus();
            }
          });
          previewEl.replaceChildren(diag);
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
        const span = document.createElement('span');
        span.className = 'text-danger text-xs';
        span.textContent = err.message || 'Parse failed';
        previewEl.replaceChildren(span);
      }
    }, 180);
  };

  inputEl.addEventListener('input', update);
  formatEl.addEventListener('change', update);
  updateHintAndBanner();
  return { refresh: update };
}
