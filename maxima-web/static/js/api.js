/**
 * Typed client API module for maxima-rs Web Service.
 */

export async function fetchHealth() {
  const res = await fetch('/api/health');
  if (!res.ok) throw new Error(`Health check failed: ${res.status}`);
  return res.json();
}

export async function fetchOperations() {
  const res = await fetch('/api/operations');
  if (!res.ok) throw new Error(`Failed to load operations: ${res.status}`);
  return res.json();
}

export async function fetchFunctions() {
  const res = await fetch('/api/functions');
  if (!res.ok) throw new Error(`Failed to load functions catalog: ${res.status}`);
  return res.json();
}

export async function fetchRules() {
  const res = await fetch('/api/rules');
  if (!res.ok) throw new Error(`Failed to load rules: ${res.status}`);
  return res.json();
}

async function readJsonSafe(res) {
  const text = await res.text();
  if (!text) return {};
  try {
    return JSON.parse(text);
  } catch (_) {
    return { error: text };
  }
}

export async function compute(payload) {
  const res = await fetch('/api/compute', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload)
  });
  const data = await readJsonSafe(res);
  if (!res.ok) {
    const msg = data.error || `Compute failed with HTTP ${res.status}`;
    const err = new Error(msg);
    err.status = res.status;
    err.data = data;
    throw err;
  }
  return data;
}

export async function parse(payload, signal = undefined) {
  const res = await fetch('/api/parse', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
    signal
  });
  const data = await readJsonSafe(res);
  if (!res.ok) {
    const msg = data.error || `Parse failed with HTTP ${res.status}`;
    const err = new Error(msg);
    err.status = res.status;
    err.data = data;
    throw err;
  }
  return data;
}
