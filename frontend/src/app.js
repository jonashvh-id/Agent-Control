import { api, setCsrf } from './services/api.js';
import { el, errorText } from './components/elements.js';
import { shell } from './components/shell.js';
import { loginPage } from './pages/login.js';
import { overview } from './pages/overview.js';
import { agentsPage } from './features/agents.js';
import { policiesPage } from './features/policies.js';
import { approvalsPage } from './features/approvals.js';
import { auditPage } from './pages/audit.js';
import { integrationsPage } from './pages/integrations.js';
import { settingsPage } from './pages/settings.js';
import { securityPage } from './pages/security.js';
const root = document.querySelector('#app');
let actor, layout, generation = 0;
const pages = { overview: () => overview(actor), agents: () => agentsPage(render,actor), policies: () => policiesPage(render,actor),
  approvals: () => approvalsPage(render), audit:auditPage, integrations:integrationsPage, settings:settingsPage, security:() => securityPage(render,actor) };
async function render() {
  if (!actor || !layout) return;
  const version = ++generation;
  const page = location.hash.slice(1) || 'overview';
  for (const item of layout.nav.querySelectorAll('a')) {
    item.classList.toggle('active',item.dataset.page===page);
    if (item.dataset.page===page) item.setAttribute('aria-current','page'); else item.removeAttribute('aria-current');
  }
  layout.content.replaceChildren(el('p',{class:'loading'},'Loading…'));
  try { const result = await (pages[page] || pages.overview)(); if(version===generation && actor) layout.content.replaceChildren(result); }
  catch(error) { if(version===generation && actor) layout.content.replaceChildren(errorText(error)); }
}
function signIn(user) {
  actor=user; setCsrf(user.csrf); layout=shell(user,async () => {
    try { await api('/auth/logout',{method:'POST'}); signOut(); }
    catch(error) { layout.content.prepend(errorText(error)); }
  }); root.replaceChildren(layout.node); render();
}
function signOut() {
  ++generation; actor=null; layout=null; setCsrf('');
  document.querySelectorAll('dialog').forEach(dialog => { dialog.close(); dialog.remove(); });
  root.replaceChildren(loginPage(signIn));
}
window.addEventListener('hashchange',render);
window.addEventListener('session-expired',signOut);
try { signIn(await api('/auth/me')); } catch { signOut(); }
