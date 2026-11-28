/**
 * Interactive Notebook Stream Model (In[n] / Out[n]) with Onboarding,
 * Per-Cell Edit/Delete/Export, Active-Tab Copy, Mode-Aware Use-As-Input,
 * and Error-Span Fix-It Diagnostics.
 */
import { buildErrorDiagnosticEl } from './fixit.js';

let cellCounter = 0;
const cells = [];

export const STARTER_EXAMPLES = [
  {
    category: 'Algebra',
    title: 'Trig Identity',
    expr: 'sin(x)^2 + cos(x)^2',
    op: 'simplify',
    params: null
  },
  {
    category: 'Algebra',
    title: 'Binomial Expansion',
    expr: '(x + 1)^4',
    op: 'expand',
    params: null
  },
  {
    category: 'Algebra',
    title: 'Cubic Factorization',
    expr: 'x^3 - 6*x^2 + 11*x - 6',
    op: 'factor',
    params: null
  },
  {
    category: 'Algebra',
    title: 'Partial Fractions',
    expr: '1/(x^2 - 1)',
    op: 'partfrac',
    params: { variable: 'x' }
  },
  {
    category: 'Calculus',
    title: 'Product Derivative',
    expr: 'sin(x) * cos(x)',
    op: 'differentiate',
    params: { variable: 'x' }
  },
  {
    category: 'Calculus',
    title: 'Integration by Parts',
    expr: 'x * exp(x)',
    op: 'integrate',
    params: { variable: 'x' }
  },
  {
    category: 'Calculus',
    title: 'Definite Integral',
    expr: '1/x^2',
    op: 'defint',
    params: { variable: 'x', lower: '1', upper: '2' }
  },
  {
    category: 'Calculus',
    title: 'Sinc Limit',
    expr: 'sin(x)/x',
    op: 'limit',
    params: { variable: 'x', point: '0' }
  },
  {
    category: 'Calculus',
    title: 'Taylor Series',
    expr: 'exp(x)',
    op: 'taylor',
    params: { variable: 'x', center: '0', order: '5' }
  },
  {
    category: 'Solvers',
    title: 'Quadratic Roots',
    expr: 'x^2 - 5*x + 6',
    op: 'solve',
    params: { variable: 'x' }
  },
  {
    category: 'Matrix',
    title: 'Bareiss Determinant',
    expr: 'matrix([1, 2], [3, 4])',
    op: 'determinant',
    params: null
  },
  {
    category: 'Codegen',
    title: 'Rosenbrock Rust Gradient',
    expr: '(1 - x)^2 + 100*(y - x^2)^2',
    op: 'rust_gradient',
    params: null
  }
];

export function initNotebook({
  container,
  onUseAsInput,
  onEditCell,
  onRunStarter,
  onApplyFix
}) {
  const showEmpty = () => {
    renderOnboarding(container, onRunStarter);
  };

  showEmpty();

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
      renderCell(container, cellData, {
        onUseAsInput,
        onEditCell,
        onApplyFix,
        onDelete: (idx) => {
          const pos = cells.findIndex(c => c.index === idx);
          if (pos !== -1) cells.splice(pos, 1);
          const el = container.querySelector(`#cell-${idx}`);
          if (el) el.remove();
          if (cells.length === 0) showEmpty();
        }
      });
      return cellData;
    },
    clear() {
      cells.length = 0;
      cellCounter = 0;
      showEmpty();
    },
    getCells() {
      return [...cells];
    }
  };
}

function renderOnboarding(container, onRunStarter) {
  const panel = document.createElement('div');
  panel.className = 'empty-notebook-state onboarding-panel';

  const header = document.createElement('div');
  header.className = 'onboarding-header';
  header.innerHTML = `
    <h2 style="font-size:1.125rem; font-weight:700; color:var(--text-main);">Welcome to maxima-rs Symbolic Notebook</h2>
    <p class="text-xs text-muted mt-1">Click any starter example below to run it immediately, or type your own expression on the left.</p>
  `;
  panel.appendChild(header);

  const grid = document.createElement('div');
  grid.className = 'starter-grid';

  STARTER_EXAMPLES.forEach((item, idx) => {
    const card = document.createElement('button');
    card.type = 'button';
    card.className = 'starter-card';
    card.dataset.starterIdx = String(idx);

    const cat = document.createElement('span');
    cat.className = 'starter-cat';
    cat.textContent = item.category;

    const title = document.createElement('span');
    title.className = 'starter-title';
    title.textContent = item.title;

    const expr = document.createElement('code');
    expr.className = 'starter-expr';
    expr.textContent = item.expr;

    card.appendChild(cat);
    card.appendChild(title);
    card.appendChild(expr);

    card.addEventListener('click', () => {
      if (typeof onRunStarter === 'function') {
        onRunStarter(item);
      }
    });

    grid.appendChild(card);
  });
  panel.appendChild(grid);

  const syntaxCard = document.createElement('div');
  syntaxCard.className = 'syntax-card';
  syntaxCard.innerHTML = `
    <div class="text-xs font-bold uppercase text-muted mb-2">Quick Syntax Guide</div>
    <div class="syntax-grid">
      <div class="syntax-item"><code>2*x^3</code><span>Powers &amp; explicit multiply</span></div>
      <div class="syntax-item"><code>%pi, %e, %i</code><span>Exact symbolic constants</span></div>
      <div class="syntax-item"><code>a: x^2 + 1</code><span>Save variable in session</span></div>
      <div class="syntax-item"><code>%, %o1</code><span>Reference last / N-th output</span></div>
      <div class="syntax-item"><code>assume(x &gt; 0)</code><span>Add domain assumption</span></div>
      <div class="syntax-item"><code>kill(all)</code><span>Reset session bindings &amp; facts</span></div>
    </div>
  `;
  panel.appendChild(syntaxCard);

  container.replaceChildren(panel);
}

function renderCell(container, cellData, callbacks) {
  const emptyState = container.querySelector('.empty-notebook-state');
  if (emptyState) emptyState.remove();

  const { index, request, response, durationMs } = cellData;
  const cell = document.createElement('article');
  cell.className = 'notebook-cell';
  cell.id = `cell-${index}`;

  const statusClass = getStatusClass(response.status);
  const statusLabel = response.status || 'ok';
  const verifiedBadge = response.verified === true
    ? '<span class="badge badge-verified" title="Mathematically verified by differentiation or root substitution">✓ verified</span>'
    : response.verified === false
      ? '<span class="badge badge-unevaluated" title="Verification could not confirm or unevaluated">? unverified</span>'
      : '';

  const rulesList = response.provenance || (response.extra && response.extra.rules) || [];

  cell.innerHTML = `
    <div class="cell-header">
      <div class="flex items-center gap-2 flex-wrap">
        <span class="cell-prompt">In[${index}]</span>
        <span class="badge bg-subtle text-muted border border-app">${escapeHtml(request.operation)}</span>
        <span class="badge ${statusClass}">${escapeHtml(statusLabel)}</span>
        ${verifiedBadge}
      </div>
      <div class="flex items-center gap-1 flex-wrap text-xs text-muted">
        <span class="px-2">${Number(durationMs) || 0}ms</span>
        <button type="button" class="btn btn-sm btn-secondary edit-cell-btn" title="Load back into editor to edit and re-run">Edit</button>
        <button type="button" class="btn btn-sm btn-secondary use-as-input-btn" title="Promote output expression to active input">Use as input</button>
        <button type="button" class="btn btn-sm btn-secondary copy-cell-btn" title="Copy active tab content">Copy</button>
        <button type="button" class="btn btn-sm btn-secondary export-md-btn" title="Export cell as Markdown">.md</button>
        <button type="button" class="btn btn-sm btn-secondary export-tex-btn" title="Export cell as LaTeX">.tex</button>
        <button type="button" class="btn btn-sm btn-secondary delete-cell-btn" aria-label="Delete cell ${index}" title="Delete this cell">&times;</button>
      </div>
    </div>
    <div class="cell-input-body">${escapeHtml(request.expression)}</div>
    <div class="cell-output-body">
      ${renderWarnings(response.warnings)}
      <div class="tab-bar" role="tablist">
        <button type="button" class="tab-btn active" role="tab" aria-selected="true" data-tab="typeset">Typeset</button>
        <button type="button" class="tab-btn" role="tab" aria-selected="false" data-tab="maxima">Maxima</button>
        <button type="button" class="tab-btn" role="tab" aria-selected="false" data-tab="latex">LaTeX</button>
        ${response.rust_code ? '<button type="button" class="tab-btn" role="tab" aria-selected="false" data-tab="rust">Rust</button>' : ''}
        ${rulesList.length ? '<button type="button" class="tab-btn" role="tab" aria-selected="false" data-tab="rules">Rules (' + rulesList.length + ')</button>' : ''}
      </div>
      <div class="tab-content">
        <div class="tab-pane tab-typeset typeset-view"></div>
        <div class="tab-pane tab-maxima code-view" style="display:none;">${escapeHtml(response.result || '')}</div>
        <div class="tab-pane tab-latex code-view" style="display:none;">${escapeHtml(response.result_latex || '')}</div>
        ${response.rust_code ? `<div class="tab-pane tab-rust code-view" style="display:none;">${escapeHtml(response.rust_code)}</div>` : ''}
        ${rulesList.length ? `<div class="tab-pane tab-rules code-view" style="display:none;">${renderProvenance(rulesList)}</div>` : ''}
      </div>
    </div>
  `;

  // Render Typeset or Error Diagnostic
  const typesetContainer = cell.querySelector('.tab-typeset');
  if (response.status === 'error') {
    const diag = buildErrorDiagnosticEl({
      expr: request.expression,
      message: response.error || response.result || 'Computation error',
      span: response.error_span,
      operation: request.operation,
      onApplyFix: callbacks.onApplyFix
    });
    typesetContainer.replaceChildren(diag);
  } else {
    renderTypesetOutput(typesetContainer, response);
  }

  // Tab switching logic
  let activeTab = 'typeset';
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
      tabBtns.forEach(b => {
        b.classList.remove('active');
        b.setAttribute('aria-selected', 'false');
      });
      btn.classList.add('active');
      btn.setAttribute('aria-selected', 'true');
      activeTab = btn.dataset.tab;
      Object.entries(tabPanes).forEach(([key, pane]) => {
        if (pane) pane.style.display = (key === activeTab) ? 'block' : 'none';
      });
    });
  });

  // Per-cell action buttons
  cell.querySelector('.edit-cell-btn').addEventListener('click', () => {
    if (callbacks.onEditCell) callbacks.onEditCell(request);
  });

  cell.querySelector('.delete-cell-btn').addEventListener('click', () => {
    if (callbacks.onDelete) callbacks.onDelete(index);
  });

  cell.querySelector('.use-as-input-btn').addEventListener('click', () => {
    if (callbacks.onUseAsInput) callbacks.onUseAsInput(response, request);
  });

  const copyBtn = cell.querySelector('.copy-cell-btn');
  copyBtn.addEventListener('click', async () => {
    let val = '';
    if (activeTab === 'maxima') val = response.result || '';
    else if (activeTab === 'latex') val = response.result_latex || '';
    else if (activeTab === 'rust') val = response.rust_code || '';
    else if (activeTab === 'rules') {
      val = rulesList.map(r => `${r.name}: ${r.identity || ''} (${r.source_file})`).join('\n');
    } else {
      val = response.result || response.result_latex || '';
    }
    if (navigator.clipboard && navigator.clipboard.writeText) {
      await navigator.clipboard.writeText(val);
    }
    copyBtn.textContent = 'Copied!';
    setTimeout(() => { copyBtn.textContent = 'Copy'; }, 1500);
  });

  cell.querySelector('.export-md-btn').addEventListener('click', () => {
    const md = [
      `# Cell In[${index}] (${request.operation})`,
      '',
      '**Input:**',
      '```maxima',
      request.expression,
      '```',
      '',
      '**Result:**',
      '```maxima',
      response.result || '',
      '```',
      '',
      response.result_latex ? `$$${response.result_latex}$$` : ''
    ].join('\n');
    downloadTextFile(`maxima-cell-${index}.md`, md, 'text/markdown');
  });

  cell.querySelector('.export-tex-btn').addEventListener('click', () => {
    const tex = [
      `% maxima-rs cell In[${index}] (${request.operation})`,
      `% Input: ${request.expression}`,
      `\\[${response.result_latex || response.result || ''}\\]`
    ].join('\n');
    downloadTextFile(`maxima-cell-${index}.tex`, tex, 'application/x-tex');
  });

  container.prepend(cell);
  if (container.parentElement) {
    container.parentElement.scrollTo({ top: 0, behavior: 'smooth' });
  } else {
    cell.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }
}

function downloadTextFile(filename, content, mimeType) {
  const blob = new Blob([content], { type: `${mimeType};charset=utf-8` });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 500);
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
    const fnName = p.source_fn || p.source_function || '';
    const identity = p.identity || p.rule || '';
    return `<div class="mb-2 pb-1 border-b border-app">
      <div class="font-bold text-primary">${escapeHtml(p.name)}</div>
      <div class="text-xs text-muted">Identity: ${escapeHtml(identity)}</div>
      <div class="text-xs text-dim">Citation: ${escapeHtml(p.source_file)} (${escapeHtml(fnName)})</div>
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
