/**
 * Interactive Notebook Stream Model (In[n] / Out[n]).
 */

let cellCounter = 0;
const cells = [];

export function initNotebook({ container, onUseAsInput }) {
  return {
    addCell(request, response, durationMs) {
      cellCounter += 1;
      const cellData = {
        index: cellCounter,
        request,
        response,
        durationMs,
        timestamp: new Date()
      };
      cells.unshift(cellData);
      renderCell(container, cellData, onUseAsInput);
      return cellData;
    },
    clear() {
      cells.length = 0;
      cellCounter = 0;
      container.innerHTML = `
        <div class="empty-notebook-state p-8 text-center text-muted border-2 border-dashed border-app rounded-xl">
          <p class="font-medium">No computations in notebook yet</p>
          <p class="text-xs text-dim mt-1">Select an operation and click <strong>Compute</strong> (or press <strong>Shift+Enter</strong>) to evaluate an expression.</p>
        </div>
      `;
    },
    getCells() {
      return [...cells];
    }
  };
}

function renderCell(container, cellData, onUseAsInput) {
  const emptyState = container.querySelector('.empty-notebook-state');
  if (emptyState) emptyState.remove();

  const { index, request, response, durationMs } = cellData;
  const cell = document.createElement('div');
  cell.className = 'notebook-cell';
  cell.id = `cell-${index}`;

  // 1. Header
  const statusClass = getStatusClass(response.status);
  const statusLabel = response.status || 'ok';
  const verifiedBadge = response.verified === true
    ? '<span class="badge badge-verified" title="Mathematically verified by differentiation or root substitution">✓ verified</span>'
    : response.verified === false
      ? '<span class="badge badge-unevaluated" title="Verification could not confirm or unevaluated">? unverified</span>'
      : '';

  cell.innerHTML = `
    <div class="cell-header">
      <div class="flex items-center gap-2">
        <span class="cell-prompt">In[${index}]</span>
        <span class="badge bg-subtle text-muted border border-app">${request.operation}</span>
        <span class="badge ${statusClass}">${statusLabel}</span>
        ${verifiedBadge}
      </div>
      <div class="flex items-center gap-2 text-xs text-muted">
        <span>${durationMs}ms</span>
        <button class="btn btn-sm btn-secondary use-as-input-btn" title="Promote output expression to active input">Use as input</button>
        <button class="btn btn-sm btn-secondary copy-cell-btn" title="Copy result to clipboard">Copy</button>
      </div>
    </div>
    <div class="cell-input-body">${escapeHtml(request.expression)}</div>
    <div class="cell-output-body">
      ${renderWarnings(response.warnings)}
      <div class="tab-bar">
        <button class="tab-btn active" data-tab="typeset">Typeset</button>
        <button class="tab-btn" data-tab="maxima">Maxima</button>
        <button class="tab-btn" data-tab="latex">LaTeX</button>
        ${response.rust_code ? '<button class="tab-btn" data-tab="rust">Rust</button>' : ''}
        ${response.provenance && response.provenance.length ? '<button class="tab-btn" data-tab="rules">Rules (' + response.provenance.length + ')</button>' : ''}
      </div>
      <div class="tab-content">
        <div class="tab-pane tab-typeset typeset-view"></div>
        <div class="tab-pane tab-maxima code-view" style="display:none;">${escapeHtml(response.result || '')}</div>
        <div class="tab-pane tab-latex code-view" style="display:none;">${escapeHtml(response.result_latex || '')}</div>
        ${response.rust_code ? `<div class="tab-pane tab-rust code-view" style="display:none;">${escapeHtml(response.rust_code)}</div>` : ''}
        ${response.provenance && response.provenance.length ? `<div class="tab-pane tab-rules code-view" style="display:none;">${renderProvenance(response.provenance)}</div>` : ''}
      </div>
    </div>
  `;

  // Render Typeset math via local KaTeX
  const typesetContainer = cell.querySelector('.tab-typeset');
  renderTypesetOutput(typesetContainer, response);

  // Tab switching logic
  const tabBtns = cell.querySelectorAll('.tab-btn');
  const tabPanes = {
    typeset: cell.querySelector('.tab-typeset'),
    maxima: cell.querySelector('.tab-maxima'),
    latex: cell.querySelector('.tab-latex'),
    rust: cell.querySelector('.tab-rust'),
    rules: cell.querySelector('.tab-rules')
  };

  tabBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      tabBtns.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      const target = btn.dataset.tab;
      Object.entries(tabPanes).forEach(([key, pane]) => {
        if (pane) pane.style.display = (key === target) ? 'block' : 'none';
      });
    });
  });

  // Action buttons
  const useBtn = cell.querySelector('.use-as-input-btn');
  useBtn.addEventListener('click', () => {
    const val = response.result || request.expression;
    if (onUseAsInput) onUseAsInput(val);
  });

  const copyBtn = cell.querySelector('.copy-cell-btn');
  copyBtn.addEventListener('click', async () => {
    const val = response.result || response.result_latex || '';
    await navigator.clipboard.writeText(val);
    copyBtn.textContent = 'Copied!';
    setTimeout(() => { copyBtn.textContent = 'Copy'; }, 1500);
  });

  container.prepend(cell);
  if (container.parentElement) {
    container.parentElement.scrollTo({ top: 0, behavior: 'smooth' });
  } else {
    cell.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }
}

function renderTypesetOutput(container, response) {
  if (response.extra && response.extra.bindings && response.extra.bindings.length > 0) {
    container.innerHTML = '';
    const bindingsHeader = document.createElement('div');
    bindingsHeader.className = 'text-xs font-semibold text-muted mb-2 uppercase';
    bindingsHeader.textContent = 'Subexpressions (Let-Bindings)';
    container.appendChild(bindingsHeader);

    response.extra.bindings.forEach(b => {
      const row = document.createElement('div');
      row.className = 'py-1 border-b border-app flex items-center justify-between gap-4';
      const label = document.createElement('span');
      label.className = 'text-xs font-mono font-semibold text-primary min-w-60';
      label.textContent = `${b.name} =`;
      const math = document.createElement('div');
      math.className = 'flex-1 overflow-x-auto';
      renderMath(math, b.latex || b.expr);
      row.appendChild(label);
      row.appendChild(math);
      container.appendChild(row);
    });

    const outHeader = document.createElement('div');
    outHeader.className = 'text-xs font-semibold text-muted mt-2 mb-1 uppercase';
    outHeader.textContent = 'Result Expression';
    container.appendChild(outHeader);

    const outMath = document.createElement('div');
    outMath.className = 'overflow-x-auto py-1';
    renderMath(outMath, response.result_latex || response.result || '');
    container.appendChild(outMath);
  } else if (response.outputs && response.outputs.length > 1) {
    // Multi-output (e.g. roots or gradient components)
    container.innerHTML = '';
    response.outputs.forEach(out => {
      const row = document.createElement('div');
      row.className = 'py-1 border-b border-app flex items-center justify-between gap-4';
      const label = document.createElement('span');
      label.className = 'text-xs font-semibold text-muted min-w-60';
      label.textContent = out.label;
      const math = document.createElement('div');
      math.className = 'flex-1 overflow-x-auto';
      renderMath(math, out.result_latex);
      row.appendChild(label);
      row.appendChild(math);
      container.appendChild(row);
    });
  } else {
    renderMath(container, response.result_latex || response.result || '');
  }
}

function renderMath(element, latexString) {
  if (!latexString) {
    element.textContent = '(empty result)';
    return;
  }
  if (window.katex) {
    try {
      element.innerHTML = window.katex.renderToString(latexString, {
        displayMode: true,
        throwOnError: false
      });
      return;
    } catch (e) {
      console.warn('KaTeX render error:', e);
    }
  }
  element.textContent = latexString;
}

function getStatusClass(status) {
  switch (status) {
    case 'ok': return 'badge-ok';
    case 'partial': return 'badge-partial';
    case 'unevaluated': return 'badge-unevaluated';
    case 'divergent': return 'badge-divergent';
    case 'no_solution': return 'badge-no-solution';
    case 'error': return 'badge-error';
    default: return 'badge-ok';
  }
}

function renderWarnings(warnings) {
  if (!warnings || warnings.length === 0) return '';
  return `
    <div class="p-2 bg-warning-light text-warning border border-warning-border rounded-md text-xs flex items-center gap-2">
      <span>⚠️</span>
      <span>${warnings.map(escapeHtml).join('; ')}</span>
    </div>
  `;
}

function renderProvenance(provenance) {
  return provenance.map(p => {
    return `<div class="mb-2 pb-1 border-b border-app">
      <div class="font-bold text-primary">${escapeHtml(p.name)}</div>
      <div class="text-xs text-muted">Rule: ${escapeHtml(p.rule)}</div>
      <div class="text-xs text-dim">Citation: ${escapeHtml(p.source_file)} (${escapeHtml(p.source_function)})</div>
    </div>`;
  }).join('');
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
