// Minimal browser test runner: open tests/index.html via the dev server.
const results = [];
export function test(name, fn) {
  try {
    fn();
    results.push({ name, ok: true });
  } catch (err) {
    results.push({ name, ok: false, message: err.message });
  }
}

export function report() {
  const failed = results.filter((r) => !r.ok);
  const lines = results.map((r) => `${r.ok ? '✔' : '✘'} ${r.name}${r.ok ? '' : ` — ${r.message}`}`);
  lines.push('', `${results.length - failed.length}/${results.length} passed`);
  document.getElementById('out').textContent = lines.join('\n');
  document.title = failed.length ? `FAIL (${failed.length})` : 'PASS';
  return { passed: results.length - failed.length, failed: failed.length };
}
