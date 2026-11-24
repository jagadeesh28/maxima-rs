/**
 * Operation Palette, Dynamic Parameter Forms, and Function Composer.
 */
import { fetchOperations, fetchFunctions } from './api.js';

let activeOp = 'simplify';
let operationsCatalog = [];
let detectedVariables = [];
let undoStack = [];

export async function initPalette({
  opContainer,
  paramsContainer,
  composeSelect,
  composeWrapBtn,
  composeUndoBtn,
  composeClearBtn,
  inputEl,
  formatEl,
  onOpChanged,
  onParamsChanged
}) {
  const paletteApi = {
    setDetectedVariables(vars) {
      detectedVariables = vars;
      renderParams(paramsContainer, activeOp, onParamsChanged);
    },
    getActiveOperation() {
      return activeOp;
    },
    getParams() {
      return collectParams(paramsContainer);
    },
    setActiveOperation(opId, customParams = null) {
      activeOp = opId;
      const btns = opContainer.querySelectorAll('.op-btn');
      btns.forEach(b => {
        b.classList.toggle('active', b.dataset.op === opId);
      });
      renderParams(paramsContainer, activeOp, onParamsChanged, customParams);
      if (onOpChanged) onOpChanged(activeOp);
    }
  };

  try {
    const [opsData, funcsData] = await Promise.all([
      fetchOperations(),
      fetchFunctions()
    ]);
    operationsCatalog = Array.isArray(opsData) ? opsData : (opsData.operations || []);
    renderOperations(opContainer, operationsCatalog, (opId) => paletteApi.setActiveOperation(opId));
    const functionsList = Array.isArray(funcsData) ? funcsData : (funcsData.functions || []);
    renderComposer(composeSelect, functionsList);
    // Render initial active operation params
    paletteApi.setActiveOperation(activeOp);
  } catch (err) {
    console.error('Failed to initialize palette:', err);
  }

  // Composer button handlers
  if (composeWrapBtn) {
    composeWrapBtn.addEventListener('click', () => {
      wrapCurrentExpression(inputEl, formatEl, composeSelect.value, composeSelect);
    });
  }

  if (composeUndoBtn) {
    composeUndoBtn.addEventListener('click', () => {
      if (undoStack.length > 0) {
        inputEl.value = undoStack.pop();
        inputEl.dispatchEvent(new Event('input'));
      }
    });
  }

  if (composeClearBtn) {
    composeClearBtn.addEventListener('click', () => {
      if (inputEl.value) {
        undoStack.push(inputEl.value);
        inputEl.value = '';
        inputEl.dispatchEvent(new Event('input'));
      }
    });
  }

  return paletteApi;
}

const CATEGORY_NAMES = {
  algebra: 'Algebra',
  calculus: 'Calculus',
  solve: 'Solvers',
  vector: 'Vector & Multivariable',
  codegen: 'Code Generation',
  eval: 'Evaluation',
  reference: 'Reference & Inspection'
};

function renderOperations(container, operations, onSelectOp) {
  container.innerHTML = '';

  // Group operations by category
  const groups = {};
  operations.forEach(op => {
    const cat = op.category || 'other';
    if (!groups[cat]) groups[cat] = [];
    groups[cat].push(op);
  });

  const categoryOrder = ['algebra', 'calculus', 'solve', 'vector', 'codegen', 'eval', 'reference'];
  const allCats = [
    ...categoryOrder.filter(c => groups[c]),
    ...Object.keys(groups).filter(c => !categoryOrder.includes(c))
  ];

  allCats.forEach(cat => {
    const ops = groups[cat];
    if (!ops || ops.length === 0) return;

    const groupDiv = document.createElement('div');
    groupDiv.className = 'op-category-group';

    const titleDiv = document.createElement('div');
    titleDiv.className = 'op-category-title';
    titleDiv.textContent = CATEGORY_NAMES[cat] || cat;
    groupDiv.appendChild(titleDiv);

    const gridDiv = document.createElement('div');
    gridDiv.className = 'op-grid';

    ops.forEach(op => {
      const btn = document.createElement('button');
      btn.className = `op-btn ${op.id === activeOp ? 'active' : ''}`;
      btn.dataset.op = op.id;
      btn.textContent = op.name || op.label || op.id;
      btn.title = op.description || '';
      btn.addEventListener('click', () => {
        if (onSelectOp) onSelectOp(op.id);
      });
      gridDiv.appendChild(btn);
    });

    groupDiv.appendChild(gridDiv);
    container.appendChild(groupDiv);
  });
}

export function renderParams(container, opId, onParamsChanged, customParams = null) {
  container.innerHTML = '';
  const op = operationsCatalog.find(o => o.id === opId);
  const paramsSection = document.getElementById('paramsSection');
  const activeOpBadge = document.getElementById('activeOpBadge');

  if (!op || !op.params || op.params.length === 0) {
    container.style.display = 'none';
    if (paramsSection) paramsSection.style.display = 'none';
    return;
  }

  if (activeOpBadge) {
    activeOpBadge.textContent = op.name || op.id;
  }
  if (paramsSection) {
    paramsSection.style.display = 'block';
  }

  container.style.display = 'flex';
  container.className = 'flex flex-wrap gap-3 p-3 bg-subtle rounded-lg border border-app mt-2';

  op.params.forEach(param => {
    const field = document.createElement('div');
    field.className = 'flex-1 min-w-120 flex flex-col gap-1';

    const label = document.createElement('label');
    label.className = 'text-xs font-semibold text-muted uppercase';
    label.textContent = param.name === 'variable'
      ? 'Variable (w.r.t.)'
      : (param.name === 'lower'
        ? 'Lower Limit (a)'
        : (param.name === 'upper'
          ? 'Upper Limit (b)'
          : (param.name === 'center'
            ? 'Expansion Point (x0)'
            : (param.name === 'order'
              ? 'Series Order (N)'
              : param.name))));

    const defaultValue = (customParams && customParams[param.name] !== undefined)
      ? customParams[param.name]
      : ((param.default !== undefined && param.default !== null)
        ? param.default
        : param.default_value);

    const paramType = param.type || param.param_type;

    let input;
    if (param.name === 'variable') {
      input = document.createElement('input');
      input.type = 'text';
      input.name = param.name;
      const initialVal = (customParams && customParams.variable)
        || (detectedVariables.length > 0 ? detectedVariables[0] : (defaultValue || 'x'));
      input.value = initialVal;
      input.placeholder = 'e.g. x';
    } else if (param.name === 'variables') {
      input = document.createElement('input');
      input.type = 'text';
      input.name = param.name;
      const initialVal = (customParams && customParams.variables)
        || (detectedVariables.length > 0 ? detectedVariables.join(', ') : (defaultValue || 'x'));
      input.value = initialVal;
      input.placeholder = 'e.g. ' + (detectedVariables.length > 0 ? detectedVariables.join(', ') : 'x');
    } else {
      input = document.createElement('input');
      input.type = (paramType === 'integer' || paramType === 'int') ? 'number' : 'text';
      input.name = param.name;
      input.placeholder = param.description || param.name;
      if (defaultValue !== null && defaultValue !== undefined) {
        input.value = defaultValue;
      }
    }

    input.dataset.param = param.name;
    input.className = 'text-xs py-1 px-2 bg-input border border-app rounded';
    input.addEventListener('input', () => {
      if (onParamsChanged) onParamsChanged();
    });
    input.addEventListener('change', () => {
      if (onParamsChanged) onParamsChanged();
    });

    field.appendChild(label);
    field.appendChild(input);
    container.appendChild(field);
  });
}

export function collectParams(container) {
  const params = {};
  if (!container) return params;
  const inputs = container.querySelectorAll('[data-param]');
  inputs.forEach(inp => {
    const name = inp.dataset.param;
    let val = inp.value.trim();
    if (inp.type === 'number') {
      const parsed = parseInt(val, 10);
      params[name] = isNaN(parsed) ? val : parsed;
    } else {
      params[name] = val;
    }
  });
  return params;
}

function renderComposer(select, functions) {
  select.innerHTML = '<option value="">-- Choose wrapper --</option>';
  const groups = {
    unary: document.createElement('optgroup'),
    binary: document.createElement('optgroup'),
    cas: document.createElement('optgroup')
  };
  groups.unary.label = 'Elementary Functions';
  groups.binary.label = 'Operators';
  groups.cas.label = 'CAS Transforms';

  functions.forEach(fn => {
    const opt = document.createElement('option');
    opt.value = fn.name;
    opt.textContent = fn.name;
    opt.dataset.kind = fn.kind;
    opt.dataset.maxima = fn.maxima_template;
    opt.dataset.latex = fn.latex_template;
    if (groups[fn.kind]) {
      groups[fn.kind].appendChild(opt);
    } else {
      select.appendChild(opt);
    }
  });

  if (groups.unary.children.length) select.appendChild(groups.unary);
  if (groups.cas.children.length) select.appendChild(groups.cas);
  if (groups.binary.children.length) select.appendChild(groups.binary);
}

function wrapCurrentExpression(inputEl, formatEl, fnName, selectEl) {
  if (!fnName) return;
  const current = inputEl.value;
  undoStack.push(current);

  const isLatex = formatEl.value === 'latex';
  const start = inputEl.selectionStart;
  const end = inputEl.selectionEnd;
  const selectedText = (start !== end) ? current.substring(start, end) : current;

  const chosenOpt = selectEl ? selectEl.querySelector(`option[value="${fnName}"]`) : null;
  const template = chosenOpt ? (isLatex ? chosenOpt.dataset.latex : chosenOpt.dataset.maxima) : null;

  let wrapped;
  if (template && template.includes('{}')) {
    wrapped = template.replace('{}', selectedText || 'x');
  } else if (isLatex) {
    wrapped = `\\${fnName}{${selectedText || 'x'}}`;
  } else {
    wrapped = `${fnName}(${selectedText || 'x'})`;
  }

  if (start !== end) {
    inputEl.value = current.substring(0, start) + wrapped + current.substring(end);
  } else {
    inputEl.value = wrapped;
  }

  inputEl.dispatchEvent(new Event('input'));
}
