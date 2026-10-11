export function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (key.startsWith('on') && typeof value === 'function') node.addEventListener(key.slice(2).toLowerCase(), value);
    else if (key === 'class') node.className = value;
    else if (key === 'text') window.AgentControlDom.setText(node, value);
    else if (key === 'disabled') node.disabled = Boolean(value);
    else if (value === false || value == null) continue;
    else if (value === true) node.setAttribute(key, '');
    else node.setAttribute(key, value);
  }
  for (const child of children.flat()) if (child != null) node.append(child instanceof Node ? child : String(child));
  return node;
}
export const button = (label, action, style = 'primary') => el('button', { class: `button ${style}`, type: 'button', onClick: action }, label);
export const badge = (value, style = '') => el('span', { class: `badge ${style || value}` }, value);
export function heading(title, subtitle, action) {
  return el('header', { class: 'page-heading' }, el('div', {}, el('div', { class: 'eyebrow' }, 'CONTROL PLANE'), el('h1', {}, title), el('p', {}, subtitle)), action);
}
export function empty(title, description) {
  return el('div', { class: 'empty' }, el('div', { class: 'empty-mark' }, '◇'), el('h3', {}, title), el('p', {}, description));
}
export function table(columns, rows) {
  return el('div', { class: 'table-wrap' }, el('table', {}, el('thead', {}, el('tr', {}, columns.map(c => el('th', {}, c)))),
    el('tbody', {}, rows.map(row => el('tr', {}, row.map(value => el('td', {}, value)))))));
}
export function date(value) { return value ? new Date(value).toLocaleString() : 'No activity yet'; }
export function panel(title, content, label = '') {
  return el('section', { class: 'panel' }, el('div', { class: 'panel-heading' }, el('h2', {}, title), label && badge(label)), content);
}
export function errorText(error) { return el('p', { class: 'error', role: 'alert' }, error.message.replaceAll('_', ' ')); }
