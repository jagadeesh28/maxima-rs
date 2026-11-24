/**
 * URL Hash State & LocalStorage History Sync.
 */

const STORAGE_KEY = 'maxima_webui_state_v1';

export function saveState({ expression, input_format, operation, fast_math, params }) {
  try {
    const state = { expression, input_format, operation, fast_math, params };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));

    // Update URL hash without polluting browser history
    const paramsQuery = new URLSearchParams();
    if (expression) paramsQuery.set('expr', expression);
    if (operation) paramsQuery.set('op', operation);
    if (input_format && input_format !== 'maxima') paramsQuery.set('fmt', input_format);
    if (fast_math) paramsQuery.set('fast', '1');

    const hash = paramsQuery.toString();
    if (hash) {
      window.history.replaceState(null, '', `#${hash}`);
    } else {
      window.history.replaceState(null, '', window.location.pathname);
    }
  } catch (e) {
    console.warn('Failed to save state to localStorage/hash:', e);
  }
}

export function loadInitialState() {
  // 1. Try URL Hash first
  if (window.location.hash && window.location.hash.length > 1) {
    try {
      const q = new URLSearchParams(window.location.hash.slice(1));
      return {
        expression: q.get('expr') || '',
        operation: q.get('op') || 'simplify',
        input_format: q.get('fmt') || 'maxima',
        fast_math: q.get('fast') === '1',
        params: {}
      };
    } catch (e) {
      console.warn('Failed to parse URL hash:', e);
    }
  }

  // 2. Try LocalStorage
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        expression: parsed.expression || '',
        operation: parsed.operation || 'simplify',
        input_format: parsed.input_format || 'maxima',
        fast_math: !!parsed.fast_math,
        params: parsed.params || {}
      };
    }
  } catch (e) {
    console.warn('Failed to read localStorage:', e);
  }

  // 3. Fallback defaults
  return {
    expression: 'sin(x)^2 + cos(x)^2',
    operation: 'simplify',
    input_format: 'maxima',
    fast_math: false,
    params: {}
  };
}
