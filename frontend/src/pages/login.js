import { api, setCsrf } from '../services/api.js';
import { el, errorText } from '../components/elements.js';
import { input } from '../components/dialog.js';
export function loginPage(onLogin) {
  const errors = el('div', { 'aria-live': 'polite' });
  const submit = el('button', { class: 'button primary', type: 'submit' }, 'Sign in →');
  const form = el('form', { class: 'login-card', onSubmit: async event => {
    event.preventDefault(); submit.disabled = true; errors.replaceChildren();
    const values = Object.fromEntries(new FormData(form));
    try { const actor = await api('/auth/login', { method: 'POST', body: values });
      setCsrf(actor.csrf); form.reset(); onLogin(actor); }
    catch (error) { errors.replaceChildren(errorText(error)); }
    finally { submit.disabled = false; }
  } }, el('div', { class: 'brand-icon large' }, 'A'), el('div', { class: 'eyebrow' }, 'AGENT CONTROL'),
  el('h1', {}, 'Your agents. Your rules.'), el('p', {}, 'Sign in to your organization’s control plane.'),
  input('Username', 'username', { autocomplete: 'username', maxlength: '80' }),
  input('Password', 'password', { type: 'password', autocomplete: 'current-password', maxlength: '256' }), errors, submit,
  el('p', { class: 'login-footnote' }, 'Self-hosted control plane · No default account'));
  return el('main', { class: 'login-layout' }, form, el('p', { class: 'login-caption' }, 'Govern the connections routed through your gateway.'));
}
