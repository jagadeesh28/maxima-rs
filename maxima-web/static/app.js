const API = location.origin + '/api';

const els = {
  expr: document.getElementById('exprInput'),
  format: document.getElementById('inputFormat'),
  operation: document.getElementById('operation'),
  variables: document.getElementById('variablesInput'),
  env: document.getElementById('envInput'),
  varsGroup: document.getElementById('varsGroup'),
  envGroup: document.getElementById('envGroup'),
  computeBtn: document.getElementById('computeBtn'),
  emptyState: document.getElementById('emptyState'),
  outputCard: document.getElementById('outputCard'),
  mathDisplay: document.getElementById('mathDisplay'),
  elapsedBadge: document.getElementById('elapsedBadge'),
  rawResult: document.getElementById('rawResult'),
  varsDetected: document.getElementById('varsDetected'),
  codeGroup: document.getElementById('codeGroup'),
  rustCode: document.getElementById('rustCode'),
  cseGroup: document.getElementById('cseGroup'),
  cseBindings: document.getElementById('cseBindings'),
  errorBox: document.getElementById('errorBox'),
  errorMsg: document.getElementById('errorMsg'),
  themeToggle: document.getElementById('themeToggle'),
  statusBadge: document.getElementById('statusBadge')
};

const OP_NEEDS_VARS = new Set(['differentiate','gradient','jacobian','hessian','rust_code']);
const OP_NEEDS_ENV = new Set(['evaluate']);

function setLoading(loading) {
  els.computeBtn.disabled = loading;
  els.computeBtn.querySelector('span').textContent = loading ? 'Computing…' : 'Compute';
}

function showError(msg) {
  els.errorBox.classList.remove('hidden');
  els.errorMsg.textContent = msg || 'An unknown error occurred';
  els.outputCard.classList.add('hidden');
  els.emptyState.classList.remove('hidden');
}

function hideError() { els.errorBox.classList.add('hidden'); }

function setVarsVisibility() {
  const op = els.operation.value;
  els.varsGroup.classList.toggle('hidden', !OP_NEEDS_VARS.has(op));
  els.envGroup.classList.toggle('hidden', !OP_NEEDS_ENV.has(op));
}

function parseEnv(s) {
  const out = {};
  if (!s.trim()) return out;
  for (const part of s.split(',')) {
    const m = part.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([^=]+?)\s*$/);
    if (m) out[m[1]] = m[2];
  }
  return out;
}

function renderLatex(el, latex) {
  el.innerHTML = '';
  if (!latex) { el.textContent = '(no result)'; return; }
  try {
    katex.render(latex, el, { throwOnError: false, displayMode: true, trust: true });
  } catch (e) {
    el.textContent = latex;
  }
}

function escapeHtml(s) {
  return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}

function copyText(text, btn) {
  navigator.clipboard.writeText(text).then(() => {
    const old = btn.innerHTML;
    btn.innerHTML = '<svg class="w-4 h-4 text-green-500" fill="currentColor" viewBox="0 0 24 24"><path fill-rule="evenodd" d="M22 11c0-1.2-1.074-2.167-2-2.388V7c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2v-1.612c.926.221 2 1.188 2 2.388 0 1.21-.854 2.237-2 2.438V21c0 1.1-.9 2-2 2H5c-1.1 0-2-.9-2-2v-1.562c-1.145-.201-2-1.228-2-2.438 0-1.2 1.074-2.167 2-2.388V7c0-1.1.9-2 2-2h6.612C19.926 8.833 22 9.796 22 11m-7-8a1 1 0 00-1 1v3a1 1 0 001 1h3a1 1 0 001-1V3a1 1 0 00-1-1H9z" clip-rule="evenodd"></path></svg>';
    setTimeout(() => btn.innerHTML = old, 1200);
  });
async function fetchOperations() {
  try {
    const res = await fetch(API + '/operations');
    const ops = await res.json();
    const sel = els.operation;
    sel.innerHTML = '';
    for (const op of ops) {
      const o = document.createElement('option');
      o.value = op.id;
      o.textContent = op.name;
      o.title = op.description;
      sel.appendChild(o);
    }
    setVarsVisibility();
  } catch (e) {
    console.warn('could not fetch operations', e);
  }
}

async function checkHealth() {
  try {
    const res = await fetch(API + '/health');
    if (res.ok) {
      els.statusBadge.textContent = 'server connected';
      els.statusBadge.className = 'ml-3 px-2.5 py-0.5 rounded text-xs font-medium bg-green-50 dark:bg-green-900/30 text-green-700 dark:text-green-400 border border-green-100 dark:border-green-800';
    } else {
      throw new Error('bad status');
    }
  } catch (e) {
    els.statusBadge.textContent = 'server unreachable';
    els.statusBadge.className = 'ml-3 px-2.5 py-0.5 rounded text-xs font-medium bg-red-50 dark:bg-red-900/30 text-red-700 dark:text-red-400 border border-red-100 dark:border-red-800';
  }
}

function initTheme() {
  const dark = localStorage.getItem('theme') === 'dark' ||
    (!localStorage.getItem('theme') && window.matchMedia('(prefers-color-scheme: dark)').matches);
  if (dark) document.documentElement.classList.add('dark');
}
els.themeToggle.addEventListener('click', () => {
  const isDark = document.documentElement.classList.toggle('dark');
  localStorage.setItem('theme', isDark ? 'dark' : 'light');
});

els.operation.addEventListener('change', setVarsVisibility);

document.querySelectorAll('.exampleChip').forEach(btn => {
  btn.addEventListener('click', () => {
    els.expr.value = btn.dataset.expr;
    els.expr.focus();
  });
});

els.computeBtn.addEventListener('click', async () => {
  const expr = els.expr.value.trim();
  if (!expr) { els.expr.focus(); return; }
  hideError();
  setLoading(true);
  els.emptyState.classList.add('hidden');
  els.outputCard.classList.remove('hidden');
  els.mathDisplay.innerHTML = '<div class="animate-pulse rounded-full h-8 w-24 bg-slate-200 dark:bg-slate-800"></div>';
  els.rawResult.textContent = '';
  els.codeGroup.classList.add('hidden');
  els.cseGroup.classList.add('hidden');
  els.varsDetected.classList.add('hidden');
  els.varsDetected.innerHTML = '';

  const variables = els.variables.value.split(',').map(s => s.trim()).filter(Boolean);
  const env = parseEnv(els.env.value);
  const payload = {
    expression: expr,
    input_format: els.format.value,
    operation: els.operation.value,
    variables,
    variable: variables.length === 1 ? variables[0] : '',
    float_type: 'f64',
    env
  };

  try {
    const res = await fetch(API + '/compute', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify(payload)
    });
    const data = await res.json();

    if (!data.success) {
      showError(data.error || 'request failed');
      return;
    }

    const extra = data.extra || {};
    const elapsed = data.elapsed_ms != null ? data.elapsed_ms : 0;
    els.elapsedBadge.textContent = elapsed + ' ms';

    if (extra.variables && Array.isArray(extra.variables) && extra.variables.length) {
      els.varsDetected.classList.remove('hidden');
      els.varsDetected.innerHTML = '<span class="text-[10px] font-semibold text-slate-400 uppercase tracking-wider mr-2">Variables:</span>' +
        extra.variables.map(v => '<span class="text-xs rounded bg-indigo-50 dark:bg-indigo-900/30 px-2 py-0.5 text-indigo-700 dark:text-indigo-300 border border-indigo-100 dark:border-indigo-800">' + escapeHtml(v) + '</span>').join(' ');
    }

    if (extra.result !== undefined) {
      renderLatex(els.mathDisplay, extra.result_latex || '');
      els.rawResult.textContent = extra.result;
    } else if (data.result) {
      renderLatex(els.mathDisplay, data.result_latex || '');
      els.rawResult.textContent = data.result;
    } else if (extra.results) {
      els.mathDisplay.innerHTML = '';
      for (const r of extra.results) {
        const row = document.createElement('div');
        row.className = 'mb-3';
        const label = document.createElement('div');
        label.className = 'text-[10px] font-semibold text-slate-400 uppercase tracking-wider mb-1';
        label.textContent = 'd/d' + escapeHtml(r.variable || 'x');
        row.appendChild(label);
        const math = document.createElement('div');
        math.className = 'mb-1';
        row.appendChild(math);
        const raw = document.createElement('div');
        raw.className = 'mono text-xs text-slate-500 dark:text-slate-400 truncate';
        raw.textContent = r.result;
        row.appendChild(raw);
        els.mathDisplay.appendChild(row);
        renderLatex(math, r.result_latex || '');
      }
      els.rawResult.textContent = extra.results.map(r => r.result).join(', ');
    } else if (extra.matrix) {
      els.mathDisplay.innerHTML = '';
      const table = document.createElement('div');
      table.className = 'overflow-x-auto';
      const inner = document.createElement('div');
      inner.className = 'inline-block min-w-full';
      let html = '<table class="border-collapse"><tbody>';
      for (const row of extra.matrix) {
        html += '<tr>';
        for (const cell of row) {
          html += '<td class="p-1 text-center"><span class="katex-block" style="display:inline-block"></span></td>';
        }
        html += '</tr>';
      }
      html += '</tbody></table>';
      inner.innerHTML = html;
      table.appendChild(inner);
      els.mathDisplay.appendChild(table);
      const cells = inner.querySelectorAll('.katex-block');
      let ci = 0;
      for (const row of extra.matrix) {
        for (const cell of row) {
          renderLatex(cells[ci++], cell.result_latex || '');
        }
      }
      els.rawResult.textContent = extra.matrix.map(r => r.map(c => c.result).join(', ')).join('; ');
    }

    if (extra.bindings && Array.isArray(extra.bindings) && extra.bindings.length) {
      els.cseGroup.classList.remove('hidden');
      els.cseBindings.innerHTML = extra.bindings.map(b =>
        '<div class="flex items-center justify-between p-1.5 rounded border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900"><span class="mono text-xs font-semibold text-indigo-700 dark:text-indigo-400">' + escapeHtml(b.name) + '</span><span class="flex-1 ml-3 mono text-xs">' + escapeHtml(b.expr) + '</span><span class="text-[10px] text-slate-400 ml-3">' + escapeHtml(b.latex || '') + '</span></div>'
      ).join('');
    }

    if (data.rust_code || extra.rust_code) {
      els.codeGroup.classList.remove('hidden');
      els.rustCode.textContent = data.rust_code || extra.rust_code || '';
    }

  } catch (e) {
    showError(e.message || 'network error');
  } finally {
    setLoading(false);
  }
});

fetchOperations();
checkHealth();
setInterval(checkHealth, 30000);
initTheme();

}
