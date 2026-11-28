/**
 * Newbie-friendly Error Span Caret Renderer, 1-Click Fix-It Suggestions,
 * and LaTeX Auto-Detection for Maxima mode.
 */

const LATEX_HINTS = [
  '\\frac', '\\sin', '\\cos', '\\tan', '\\sqrt', '\\int', '\\sum',
  '\\pi', '\\cdot', '\\ln', '\\exp', '\\left', '\\right', '^{'
];

/**
 * Returns true if the expression in Maxima mode looks like LaTeX.
 */
export function looksLikeLatex(expr, inputFormat) {
  if (inputFormat !== 'maxima' || !expr) return false;
  return LATEX_HINTS.some(tok => expr.includes(tok));
}

/**
 * Computes 1-click Fix-It suggestions for common beginner syntax mistakes.
 * Each suggestion has `{ label, fixed, description }`.
 */
export function suggestFixes(expr, operation = '') {
  if (!expr || !expr.trim()) return [];
  const suggestions = [];
  const seen = new Set();

  const add = (label, fixed, description) => {
    if (fixed && fixed !== expr && !seen.has(fixed)) {
      seen.add(fixed);
      suggestions.push({ label, fixed, description });
    }
  };

  // 1. Python power `**` -> `^`
  if (expr.includes('**')) {
    add('Use ^ for powers', expr.replace(/\*\*/g, '^'), 'Replace ** with ^');
  }

  // 2. Implicit multiplication: `2x`, `3sin(x)`, `)x`, `)(`, `2(`
  const implicitMul = expr
    .replace(/(\d)([A-Za-z%])/g, '$1*$2')
    .replace(/(\d)\(/g, '$1*(')
    .replace(/\)\(/g, ')*(')
    .replace(/\)([A-Za-z%\d])/g, ')*$1');
  if (implicitMul !== expr) {
    add('Add * for multiplication', implicitMul, 'Insert explicit * operators');
  }

  // 3. Bare constants: `pi`, `e`, `i` -> `%pi`, `%e`, `%i`
  const bareConsts = expr
    .replace(/\bpi\b/g, '%pi')
    .replace(/(^|[^%A-Za-z_])e\b/g, '$1%e')
    .replace(/(^|[^%A-Za-z_])i\b/g, '$1%i');
  if (bareConsts !== expr && !expr.includes('%pi') && !expr.includes('%e') && !expr.includes('%i')) {
    add('Use %pi / %e / %i', bareConsts, 'Prefix Maxima constants with %');
  }

  // 4. Equation `lhs = rhs` in solve or general expression -> `(lhs) - (rhs)`
  if (expr.includes('=') && !expr.includes('<=') && !expr.includes('>=') && !expr.includes('!=') && !expr.includes(':=')) {
    const parts = expr.split('=');
    if (parts.length === 2 && parts[0].trim() && parts[1].trim()) {
      const lhs = parts[0].trim();
      const rhs = parts[1].trim();
      const fixed = rhs === '0' ? lhs : `(${lhs}) - (${rhs})`;
      add(
        operation === 'solve' ? 'Convert equation to lhs - rhs' : 'Rewrite = as (lhs) - (rhs)',
        fixed,
        'Convert equation to zero-form expression'
      );
    }
  }

  // 5. Space-separated trig/elem calls: `sin x` -> `sin(x)`
  const spaceCall = expr.replace(/\b(sin|cos|tan|sinh|cosh|tanh|asin|acos|atan|exp|ln|log|sqrt|abs)\s+([A-Za-z0-9_%]+)/g, '$1($2)');
  if (spaceCall !== expr) {
    add('Add parentheses to function call', spaceCall, 'Wrap argument in (...)');
  }

  // 6. Unbalanced parentheses
  let balance = 0;
  for (const ch of expr) {
    if (ch === '(') balance += 1;
    else if (ch === ')') balance -= 1;
  }
  if (balance > 0) {
    add('Close missing )', expr + ')'.repeat(balance), `Append ${balance} closing parenthesis`);
  } else if (balance < 0) {
    let extra = -balance;
    let trimmed = expr;
    while (extra > 0 && trimmed.endsWith(')')) {
      trimmed = trimmed.slice(0, -1);
      extra -= 1;
    }
    if (trimmed !== expr) {
      add('Remove extra )', trimmed, 'Remove unmatched closing parenthesis');
    }
  }

  return suggestions;
}

/**
 * Builds an accessible DOM block showing the escaped monospace error span caret (^~~~)
 * and clickable 1-click Fix-It buttons.
 */
export function buildErrorDiagnosticEl({ expr, message, span, operation, onApplyFix }) {
  const wrap = document.createElement('div');
  wrap.className = 'error-diag';

  const msgEl = document.createElement('div');
  msgEl.className = 'text-danger text-xs font-semibold';
  msgEl.textContent = message || 'Syntax error';
  wrap.appendChild(msgEl);

  if (expr && span && typeof span.start === 'number' && typeof span.end === 'number') {
    const start = Math.max(0, Math.min(span.start, expr.length));
    const end = Math.max(start + 1, Math.min(span.end, expr.length));
    const width = Math.max(1, end - start);
    const caretLine = ' '.repeat(start) + '^' + '~'.repeat(Math.max(0, width - 1));

    const pre = document.createElement('pre');
    pre.className = 'error-caret-pre';
    pre.textContent = `${expr}\n${caretLine}`;
    wrap.appendChild(pre);
  }

  const fixes = suggestFixes(expr, operation);
  if (fixes.length > 0 && typeof onApplyFix === 'function') {
    const row = document.createElement('div');
    row.className = 'fixit-row';
    fixes.forEach(fix => {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'fixit-btn';
      btn.title = `${fix.description}: ${fix.fixed}`;
      btn.textContent = `Fix: ${fix.label}`;
      btn.addEventListener('click', () => onApplyFix(fix.fixed));
      row.appendChild(btn);
    });
    wrap.appendChild(row);
  }

  return wrap;
}
