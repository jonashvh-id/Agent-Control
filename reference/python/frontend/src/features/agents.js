import { api } from '../services/api.js';
import { el, button, badge, table, heading, empty, panel, date } from '../components/elements.js';
import { dialog, input, showCredential } from '../components/dialog.js';
const scopes = value => value.split(',').map(s => s.trim()).filter(Boolean);
export async function agentsPage(refresh, actor) {
  if (actor.role !== 'admin') return el('div', {}, heading('Agents', 'Agent registry administration is available to administrators.'), empty('Administrator access required', 'Your reviewer account can review approval requests.'));
  const agents = await api('/agents');
  const create = () => dialog('Register an agent', [input('Name','name',{maxlength:'80'}),input('Description','description',{required:false,maxlength:'500'}),
    input('Provider','provider',{required:false,placeholder:'OpenAI, Anthropic, custom…',maxlength:'40'}),input('Runtime','runtime',{required:false,placeholder:'Agent runtime metadata',maxlength:'80'}),
    input('Assigned scopes, comma separated','scopes',{required:false,placeholder:'issues:read, issues:write'})], 'Register agent', async values => {
    await api('/agents',{method:'POST',body:{...values,scopes:scopes(values.scopes)}}); await refresh();
  });
  return el('div', {}, heading('Agents', 'Independent identities with explicit scopes and revocable credentials.', button('＋ Register agent', create)),
    panel('Agent registry', agents.length ? table(['Agent','Provider / runtime','Scopes','State',''], agents.map(a => [el('div', {},el('strong', {},a.name),el('small', {},a.description || a.id.slice(0,12))),
      el('div', {},a.provider || 'Not specified',el('small', {},a.runtime || 'Not specified')),String(a.scopes.length),badge(a.status),
      button('Manage →',() => details(a.id,refresh),'text')])) : empty('No agents registered', 'Register your first agent. Every tool starts with a deny policy.')));
}
async function details(id, refresh) {
  const agent = await api(`/agents/${id}`);
  const body = el('div', { class: 'agent-detail' }, el('h2', {},agent.name), el('p', {},agent.description),
    el('p', {},`Agent ID: ${agent.id}`), el('p', {},`Assigned scopes: ${agent.scopes.join(', ') || 'None'}`),
    el('p', {},`Last gateway execution: ${date(agent.last_used_at)}`), el('p', {},'Health monitoring is planned.'), badge(agent.status));
  const modal = el('dialog', { onClose:() => modal.remove() }, body);
  const notice = el('div', {'aria-live':'polite'}); body.append(notice);
  const act = async action => { try { await action(); modal.close(); await refresh(); } catch(e) { notice.textContent=e.message.replaceAll('_',' '); } };
  body.append(el('div', { class:'form-actions' }, ['active','paused','disabled'].map(status => button(status === 'active' ? 'Activate' : status === 'paused' ? 'Pause' : 'Disable',
    () => act(() => api(`/agents/${id}/status`,{method:'PATCH',body:{status}})),status === 'disabled' ? 'danger' : 'secondary'))));
  body.append(el('h3', {},'Credentials'), button('Issue scoped credential', () => dialog('Issue credential',
    [input('Scopes, comma separated','scopes',{required:false,placeholder:agent.scopes.join(', ')})], 'Issue credential', async values => {
      const result = await api(`/agents/${id}/credentials`,{method:'POST',body:{scopes:scopes(values.scopes)}});
      modal.close(); showCredential(result.bearer_token);
    }), 'secondary'));
  for (const credential of agent.credentials) body.append(el('div', { class:'credential-row' }, el('code', {},credential.id.slice(0,12)),badge(credential.status),
    credential.status === 'active' ? button('Revoke', () => act(() => api(`/agents/${id}/credentials/${credential.id}`,{method:'DELETE'})), 'danger') : null));
  body.append(button('Close', () => modal.close(), 'secondary'));
  document.body.append(modal); modal.showModal();
}
