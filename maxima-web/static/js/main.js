/**
 * maxima-rs WebUI Entry Point & Orchestrator (ES Module).
 */
import { fetchHealth, compute, fetchRules } from './api.js';
import { initPreview } from './preview.js';
import { initPalette } from './palette.js';
import { initNotebook } from './notebook.js';
import { saveState, loadInitialState } from './history.js';

document.addEventListener('DOMContentLoaded', async () => {
  // DOM Elements
  const themeToggle = document.getElementById('themeToggle');
  const statusBadge = document.getElementById('statusBadge');
  const exprInput = document.getElementById('exprInput');
  const inputFormatSelect = document.getElementById('inputFormat');
  const fastMathToggle = document.getElementById('fastMathToggle');
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
  const rulesModal = document.getElementById('rulesModal');
  const openRulesBtn = document.getElementById('openRulesBtn');
  const closeRulesBtn = document.getElementById('closeRulesBtn');
  const rulesList = document.getElementById('rulesList');

  // Initialize Dark Mode
  initDarkMode(themeToggle);

  // Initialize Notebook
  const notebook = initNotebook({
    container: notebookContainer,
    onUseAsInput: (expr) => {
      exprInput.value = expr;
      exprInput.dispatchEvent(new Event('input'));
      exprInput.focus();
    }
  });

  if (clearNotebookBtn) {
    clearNotebookBtn.addEventListener('click', () => notebook.clear());
  }

  // Initialize Palette
  let palette = null;
  palette = await initPalette({
    opContainer,
    paramsContainer,
    composeSelect,
    composeWrapBtn,
    composeUndoBtn,
    composeClearBtn,
    inputEl: exprInput,
    formatEl: inputFormatSelect,
    onOpChanged: (op) => {
      syncCurrentState();
    },
    onParamsChanged: () => {
      syncCurrentState();
    }
  });

  // Initialize Preview
  const preview = initPreview({
    inputEl: exprInput,
    formatEl: inputFormatSelect,
    previewEl: previewBox,
    varsContainer,
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

    computeBtn.disabled = true;
    computeBtn.classList.add('opacity-75');
    const startTime = performance.now();

    try {
      const varStr = params.variable || (params.variables ? params.variables.split(',')[0].trim() : '');
      const varsArr = params.variables
        ? params.variables.split(',').map(s => s.trim()).filter(Boolean)
        : (params.variable ? [params.variable.trim()] : []);

      const payload = {
        expression,
        input_format,
        operation,
        variable: varStr,
        variables: varsArr,
        params,
        fast_math
      };
      const res = await compute(payload);
      const durationMs = Math.round(performance.now() - startTime);
      notebook.addCell(payload, res, durationMs);
    } catch (err) {
      const durationMs = Math.round(performance.now() - startTime);
      const errResponse = {
        status: 'error',
        result: err.message,
        result_latex: `\\text{Error: } ${err.message}`,
        warnings: [err.message],
        outputs: []
      };
      notebook.addCell({ expression, operation }, errResponse, durationMs);
      showToast(err.message, 'danger');
    } finally {
      computeBtn.disabled = false;
      computeBtn.classList.remove('opacity-75');
    }
  }

  computeBtn.addEventListener('click', triggerCompute);

  // Shift+Enter to compute
  exprInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && e.shiftKey) {
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
        exprInput.value = expr;
        exprInput.dispatchEvent(new Event('input'));
      }
      if (op && palette) {
        palette.setActiveOperation(op, params);
      }
      exprInput.focus();
    });
  });

  // Rules Catalog Modal
  if (openRulesBtn && rulesModal && closeRulesBtn) {
    openRulesBtn.addEventListener('click', async () => {
      rulesModal.style.display = 'flex';
      rulesList.innerHTML = '<div class="text-sm text-muted">Loading rules from engine...</div>';
      try {
        const data = await fetchRules();
        renderRulesModal(rulesList, data.rules || []);
      } catch (err) {
        rulesList.innerHTML = `<div class="text-sm text-danger">${err.message}</div>`;
      }
    });

    closeRulesBtn.addEventListener('click', () => {
      rulesModal.style.display = 'none';
    });

    rulesModal.addEventListener('click', (e) => {
      if (e.target === rulesModal) rulesModal.style.display = 'none';
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
  container.innerHTML = rules.map(r => `
    <div class="p-3 border-b border-app flex flex-col gap-1">
      <div class="flex items-center justify-between">
        <span class="font-bold text-sm text-primary">${escapeHtml(r.name)}</span>
        <span class="text-xs text-muted">${escapeHtml(r.source_file)} (${escapeHtml(r.source_function)})</span>
      </div>
      <div class="text-xs font-mono bg-subtle p-2 rounded">${escapeHtml(r.rule)}</div>
      <div class="text-xs text-dim">${escapeHtml(r.description || '')}</div>
    </div>
  `).join('');
}

function showToast(message, type = 'info') {
  let container = document.getElementById('toastContainer');
  if (!container) {
    container = document.createElement('div');
    container.id = 'toastContainer';
    container.className = 'toast-container';
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
