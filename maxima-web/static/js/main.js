/**
 * maxima-rs WebUI Entry Point & Orchestrator (ES Module).
 */
import { fetchHealth, compute, fetchRules } from './api.js';
import { initPreview } from './preview.js';
import { initPalette } from './palette.js';
import { initNotebook } from './notebook.js';
import { saveState, loadInitialState } from './history.js';

function createEmptySessionContext() {
  return {
    last_result: null,
    outputs: {},
    bindings: {},
    facts: [],
    next_index: 1
  };
}

document.addEventListener('DOMContentLoaded', async () => {
  // DOM Elements
  const themeToggle = document.getElementById('themeToggle');
  const statusBadge = document.getElementById('statusBadge');
  const exprInput = document.getElementById('exprInput');
  const inputFormatSelect = document.getElementById('inputFormat');
  const fastMathToggle = document.getElementById('fastMathToggle');
  const fpprecInput = document.getElementById('fpprecInput');
  const syntaxHintEl = document.getElementById('syntaxHintBar');
  const latexBannerEl = document.getElementById('latexDetectBanner');
  const switchLatexBtn = document.getElementById('switchLatexBtn');
  const previewBox = document.getElementById('previewBox');
  const varsContainer = document.getElementById('varsContainer');
  const opContainer = document.getElementById('opGrid');
  const paramsContainer = document.getElementById('paramsContainer');
  const composeSelect = document.getElementById('composeFn');
  const composeWrapBtn = document.getElementById('composeWrapBtn');
  const composeUndoBtn = document.getElementById('composeUndoBtn');
  const composeClearBtn = document.getElementById('composeClearBtn');
  const computeBtn = document.getElementById('computeBtn');
  const notebookContainer = document.getElementById('notebookStream');
  const clearNotebookBtn = document.getElementById('clearNotebookBtn');
  const resetSessionBtn = document.getElementById('resetSessionBtn');
  const sessionBar = document.getElementById('sessionBar');
  const rulesModal = document.getElementById('rulesModal');
  const openRulesBtn = document.getElementById('openRulesBtn');
  const closeRulesBtn = document.getElementById('closeRulesBtn');
  const rulesList = document.getElementById('rulesList');

  let sessionContext = createEmptySessionContext();
  let palette = null;

  initDarkMode(themeToggle);

  function renderSessionBar() {
    if (!sessionBar) return;
    const bindingsCount = Object.keys(sessionContext.bindings || {}).length;
    const factsCount = (sessionContext.facts || []).length;
    const hasLast = !!sessionContext.last_result;

    if (!bindingsCount && !factsCount && !hasLast) {
      sessionBar.style.display = 'none';
      sessionBar.replaceChildren();
      return;
    }

    sessionBar.style.display = 'flex';
    const items = [];
    if (hasLast) {
      const pill = document.createElement('span');
      pill.className = 'session-pill';
      pill.textContent = `% = ${sessionContext.last_result}`;
      items.push(pill);
    }
    Object.entries(sessionContext.bindings || {}).forEach(([k, v]) => {
      const pill = document.createElement('span');
      pill.className = 'session-pill';
      pill.textContent = `${k}: ${v}`;
      items.push(pill);
    });
    (sessionContext.facts || []).forEach(f => {
      const pill = document.createElement('span');
      pill.className = 'session-pill';
      pill.textContent = `assume(${f})`;
      items.push(pill);
    });
    sessionBar.replaceChildren(...items);
  }

  // Initialize Notebook
  const notebook = initNotebook({
    container: notebookContainer,
    onUseAsInput: (response, request) => {
      const useLatex = inputFormatSelect.value === 'latex';
      const val = useLatex
        ? (response.result_latex || response.result || request.expression)
        : (response.result || request.expression);
      exprInput.value = val;
      exprInput.dispatchEvent(new Event('input'));
      exprInput.focus();
    },
    onEditCell: (request) => {
      if (request.input_format) inputFormatSelect.value = request.input_format;
      exprInput.value = request.expression || '';
      exprInput.dispatchEvent(new Event('input'));
      if (request.operation && palette) {
        palette.setActiveOperation(request.operation, request.params || null);
      }
      exprInput.focus();
    },
    onRunStarter: (item) => {
      inputFormatSelect.value = 'maxima';
      exprInput.value = item.expr;
      exprInput.dispatchEvent(new Event('input'));
      if (palette && item.op) {
        palette.setActiveOperation(item.op, item.params || null);
      }
      triggerCompute();
    },
    onApplyFix: (fixed) => {
      exprInput.value = fixed;
      exprInput.dispatchEvent(new Event('input'));
      exprInput.focus();
    }
  });

  if (clearNotebookBtn) {
    clearNotebookBtn.addEventListener('click', () => notebook.clear());
  }

  if (resetSessionBtn) {
    resetSessionBtn.addEventListener('click', () => {
      sessionContext = createEmptySessionContext();
      renderSessionBar();
      showToast('Session memory cleared (bindings, %, and assumptions reset)', 'info');
    });
  }

  // Initialize Palette
  palette = await initPalette({
    opContainer,
    paramsContainer,
    composeSelect,
    composeWrapBtn,
    composeUndoBtn,
    composeClearBtn,
    inputEl: exprInput,
    formatEl: inputFormatSelect,
    onOpChanged: () => syncCurrentState(),
    onParamsChanged: () => syncCurrentState(),
    onSubmitShortcut: () => triggerCompute()
  });

  // Initialize Preview
  const preview = initPreview({
    inputEl: exprInput,
    formatEl: inputFormatSelect,
    previewEl: previewBox,
    varsContainer,
    syntaxHintEl,
    latexBannerEl,
    switchLatexBtn,
    getActiveOperation: () => (palette ? palette.getActiveOperation() : 'simplify'),
    onVariablesDetected: (vars) => {
      if (palette) palette.setDetectedVariables(vars);
    }
  });

  // Load and apply initial state
  const state = loadInitialState();
  if (state.expression) exprInput.value = state.expression;
  if (state.input_format) inputFormatSelect.value = state.input_format;
  if (state.fast_math) fastMathToggle.checked = state.fast_math;
  if (state.operation && palette) palette.setActiveOperation(state.operation);

  preview.refresh();

  function syncCurrentState() {
    saveState({
      expression: exprInput.value.trim(),
      input_format: inputFormatSelect.value,
      operation: palette ? palette.getActiveOperation() : 'simplify',
      fast_math: fastMathToggle.checked,
      params: palette ? palette.getParams() : {}
    });
  }

  exprInput.addEventListener('input', syncCurrentState);
  inputFormatSelect.addEventListener('change', syncCurrentState);
  fastMathToggle.addEventListener('change', syncCurrentState);

  // Compute Action
  async function triggerCompute() {
    const expression = exprInput.value.trim();
    if (!expression) {
      exprInput.focus();
      return;
    }

    const operation = palette ? palette.getActiveOperation() : 'simplify';
    const params = palette ? palette.getParams() : {};
    const input_format = inputFormatSelect.value;
    const fast_math = fastMathToggle.checked;

    if (fpprecInput && operation === 'bfloat' && !params.fpprec) {
      const prec = parseInt(fpprecInput.value, 10);
      if (!isNaN(prec) && prec >= 1) params.fpprec = prec;
    }

    computeBtn.disabled = true;
    computeBtn.classList.add('opacity-75');
    const startTime = performance.now();

    try {
      const varStr = params.variable || (params.variables ? params.variables.split(',')[0].trim() : '');
      const varsArr = params.variables
        ? params.variables.split(',').map(s => s.trim()).filter(Boolean)
        : (params.variable ? [params.variable.trim()] : []);
      const env = palette && palette.getEnv ? palette.getEnv() : {};

      const payload = {
        expression,
        input_format,
        operation,
        variable: varStr,
        variables: varsArr,
        float_type: params.float_type || 'f64',
        env,
        params,
        fast_math,
        session_context: sessionContext
      };
      const res = await compute(payload);
      if (res.session_context) {
        sessionContext = res.session_context;
        renderSessionBar();
      }
      const durationMs = Math.round(performance.now() - startTime);
      notebook.addCell(payload, res, durationMs);
    } catch (err) {
      const durationMs = Math.round(performance.now() - startTime);
      const errData = err.data || {};
      const errResponse = {
        status: 'error',
        result: err.message,
        error: err.message,
        error_span: errData.error_span || null,
        result_latex: '',
        warnings: [err.message],
        outputs: []
      };
      notebook.addCell({ expression, operation, input_format, params }, errResponse, durationMs);
      showToast(err.message, 'danger');
    } finally {
      computeBtn.disabled = false;
      computeBtn.classList.remove('opacity-75');
    }
  }

  computeBtn.addEventListener('click', triggerCompute);

  // Shift+Enter or Ctrl+Enter to compute
  exprInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && (e.shiftKey || e.ctrlKey)) {
      e.preventDefault();
      triggerCompute();
    }
  });

  // Example chips handler
  document.querySelectorAll('.exampleChip').forEach(chip => {
    chip.addEventListener('click', () => {
      const expr = chip.dataset.expr;
      const op = chip.dataset.op;
      let params = null;
      if (chip.dataset.params) {
        try {
          params = JSON.parse(chip.dataset.params);
        } catch (_) {}
      }
      if (expr) {
        inputFormatSelect.value = 'maxima';
        exprInput.value = expr;
        exprInput.dispatchEvent(new Event('input'));
      }
      if (op && palette) {
        palette.setActiveOperation(op, params);
      }
      exprInput.focus();
    });
  });

  // Rules Catalog Modal with Escape close & focus trapping
  if (openRulesBtn && rulesModal && closeRulesBtn) {
    const closeModal = () => {
      rulesModal.style.display = 'none';
      openRulesBtn.focus();
    };

    openRulesBtn.addEventListener('click', async () => {
      rulesModal.style.display = 'flex';
      closeRulesBtn.focus();
      rulesList.innerHTML = '<div class="text-sm text-muted">Loading rules from engine...</div>';
      try {
        const data = await fetchRules();
        renderRulesModal(rulesList, data.rules || []);
      } catch (err) {
        const errDiv = document.createElement('div');
        errDiv.className = 'text-sm text-danger';
        errDiv.textContent = err.message || 'Failed to load rules';
        rulesList.replaceChildren(errDiv);
      }
    });

    closeRulesBtn.addEventListener('click', closeModal);

    rulesModal.addEventListener('click', (e) => {
      if (e.target === rulesModal) closeModal();
    });

    document.addEventListener('keydown', (e) => {
      if (rulesModal.style.display !== 'flex') return;
      if (e.key === 'Escape') {
        e.preventDefault();
        closeModal();
        return;
      }
      if (e.key === 'Tab') {
        const focusables = rulesModal.querySelectorAll(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        );
        if (focusables.length === 0) return;
        const first = focusables[0];
        const last = focusables[focusables.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    });
  }

  // Periodic health check
  checkServerHealth(statusBadge);
  setInterval(() => checkServerHealth(statusBadge), 15000);
});

function initDarkMode(toggleBtn) {
  const root = document.documentElement;
  const saved = localStorage.getItem('maxima_theme');
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
  const isDark = saved ? saved === 'dark' : prefersDark;

  if (isDark) {
    root.classList.add('dark');
  } else {
    root.classList.remove('dark');
  }

  if (toggleBtn) {
    toggleBtn.addEventListener('click', () => {
      const currentlyDark = root.classList.contains('dark');
      if (currentlyDark) {
        root.classList.remove('dark');
        localStorage.setItem('maxima_theme', 'light');
      } else {
        root.classList.add('dark');
        localStorage.setItem('maxima_theme', 'dark');
      }
    });
  }
}

async function checkServerHealth(badge) {
  if (!badge) return;
  try {
    const health = await fetchHealth();
    if (health.status === 'ok') {
      badge.textContent = 'Server Online';
      badge.className = 'badge badge-ok';
    } else {
      badge.textContent = 'Degraded';
      badge.className = 'badge badge-unevaluated';
    }
  } catch (err) {
    badge.textContent = 'Offline';
    badge.className = 'badge badge-error';
  }
}

function renderRulesModal(container, rules) {
  if (!rules || rules.length === 0) {
    container.innerHTML = '<div class="text-sm text-muted">No rules reported.</div>';
    return;
  }
  container.innerHTML = rules.map(r => {
    const fnName = r.source_fn || r.source_function || '';
    const identity = r.identity || r.rule || '';
    const fastBadge = r.fast_math_only
      ? '<span class="badge badge-partial">fast-math</span>'
      : '';
    return `
    <div class="p-3 border-b border-app flex flex-col gap-1">
      <div class="flex items-center justify-between gap-2">
        <span class="font-bold text-sm text-primary">${escapeHtml(r.name)}</span>
        <div class="flex items-center gap-2">
          ${fastBadge}
          <span class="text-xs text-muted">${escapeHtml(r.source_file)} (${escapeHtml(fnName)})</span>
        </div>
      </div>
      <div class="text-xs font-mono bg-subtle p-2 rounded">${escapeHtml(identity)}</div>
    </div>
  `;
  }).join('');
}

function showToast(message, type = 'info') {
  let container = document.getElementById('toastContainer');
  if (!container) {
    container = document.createElement('div');
    container.id = 'toastContainer';
    container.className = 'toast-container';
    container.setAttribute('aria-live', 'polite');
    document.body.appendChild(container);
  }

  const toast = document.createElement('div');
  toast.className = `toast border-${type}`;
  toast.textContent = message;
  container.appendChild(toast);

  setTimeout(() => {
    toast.remove();
  }, 4000);
}

function escapeHtml(str) {
  if (!str) return '';
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
