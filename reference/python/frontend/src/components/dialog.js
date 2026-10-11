import { el, button, errorText } from './elements.js';
export function input(label, name, options = {}) {
  return el('label', {}, el('span', {}, label), el('input', { name, required: true, ...options }));
}
export function select(label, name, values) {
  return el('label', {}, el('span', {}, label), el('select', { name, required: true }, values.map(([value, text]) => el('option', { value }, text))));
}
export function dialog(title, fields, submitLabel, submit) {
  const notice = el('div', { 'aria-live': 'polite' });
  const cancel = button('Cancel', () => modal.close(), 'secondary');
  const submitButton = el('button', { type: 'submit', class: 'button primary' }, submitLabel);
  const form = el('form', { onSubmit: async event => {
    event.preventDefault(); submitButton.disabled = true; notice.replaceChildren();
    try { await submit(Object.fromEntries(new FormData(form))); modal.close(); }
    catch (error) { notice.replaceChildren(errorText(error)); }
    finally { submitButton.disabled = false; }
  } }, el('h2', {}, title), fields, notice, el('div', { class: 'form-actions' }, cancel, submitButton));
  const modal = el('dialog', { onClose: () => modal.remove() }, form);
  document.body.append(modal); modal.showModal();
  return modal;
}
export function showCredential(token) {
  const field = el('textarea', { readonly: '', rows: '3', 'aria-label': 'New agent bearer credential' }, token);
  const modal = el('dialog', { onClose: () => { field.value = ''; modal.remove(); } },
    el('h2', {}, 'Save your credential'), el('p', {}, 'This credential is shown once. Store it in your secret manager. Closing this dialog removes it from the page.'),
    field, button('I have saved it', () => modal.close()));
  document.body.append(modal); modal.showModal();
}
