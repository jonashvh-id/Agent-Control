let csrf = '';
export function setCsrf(value) { csrf = value || ''; }
export async function api(path, options = {}) {
  const headers = { 'X-Control-Request': '1', ...options.headers };
  if (options.body !== undefined) headers['Content-Type'] = 'application/json';
  if (options.method && options.method !== 'GET') headers['X-CSRF-Token'] = csrf;
  const response = await fetch(`/api/v1${path}`, { ...options, headers, credentials: 'same-origin',
    body: options.body !== undefined ? JSON.stringify(options.body) : undefined });
  const result = response.status === 204 ? null : await response.json();
  if (!response.ok) {
    if (response.status === 401 && path !== '/auth/login') window.dispatchEvent(new Event('session-expired'));
    throw new Error(result?.error || 'Request failed');
  }
  return result;
}
