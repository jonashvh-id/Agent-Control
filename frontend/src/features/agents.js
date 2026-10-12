import { api } from '../services/api.js';
import { el,button,badge,table,heading,empty,panel,date } from '../components/elements.js';
import { dialog,input,select } from '../components/dialog.js';
import { scopeChoices,selectedScopes } from '../components/scopes.js';
import { applicationName,toolName,accessSummary } from '../services/explanations.js';
import { connectAgent } from './connect.js';
export async function agentsPage(refresh,actor) {
  if (actor.role !== 'admin') return el('div',{},heading('Agents','Reviewers can review approvals. Agent administration requires an administrator.'),empty('Administrator access required','Open Approvals to review pending requests.'));
  const [agents,catalog]=await Promise.all([api('/agents'),api('/catalog')]);
  const create=() => dialog('Register your agent',[
    input('Agent name','name',{maxlength:'80',placeholder:'My Codex agent'}),
    select('Agent software','runtime',[['Codex','Codex'],['Claude','Claude'],['Custom','Custom agent']]),
    input('Description','description',{required:false,maxlength:'500',placeholder:'What will this agent help you do?'}),
    scopeChoices(catalog),el('p',{class:'form-note'},'Start with a few capabilities. Every tool remains denied until you choose a policy.')
  ],'Register agent',async values => {
    const agent=await api('/agents',{method:'POST',body:{name:values.name,description:values.description,runtime:values.runtime,
      provider:values.runtime==='Codex'?'OpenAI':values.runtime==='Claude'?'Anthropic':'',scopes:selectedScopes(values)}});
    await refresh();await details(agent.id,refresh,catalog);
  });
  return el('div',{},heading('Agents','Choose who can use each application, then connect a scoped credential.',button('＋ Register agent',create)),
    agents.length ? panel('Your agents',table(['Agent','Assigned capabilities','State',''],agents.map(a => [
      el('div',{},el('strong',{},a.name),el('small',{},a.runtime || 'Custom agent'),el('small',{},a.description)),
      a.scopes.length ? a.scopes.map(s => el('span',{class:'capability'},s)) : 'None assigned',badge(a.status),
      button('Access & connection →',() => details(a.id,refresh,catalog),'text')
    ]))) : empty('Connect your first agent','Register its identity, choose tool permissions, and create a scoped credential for Codex.'));
}
async function details(id,refresh,catalog) {
  const [agent,access]=await Promise.all([api(`/agents/${id}`),api(`/agents/${id}/access`)]);
  const body=el('div',{class:'agent-detail'},el('h2',{},agent.name),el('p',{},agent.description),
    el('p',{},`Last completed action: ${date(agent.last_used_at)}`),badge(agent.status));
  const modal=el('dialog',{class:'wide-dialog',onClose:() => modal.remove()},body);
  const notice=el('div',{'aria-live':'polite'});body.append(notice);
  const act=async action => {try {await action();modal.close();await refresh();} catch(e) {notice.textContent=e.message.replaceAll('_',' ');}};
  body.append(el('h3',{},'Who can access what'),access.tools.length ? table(['Application / action','Policy','Effective access'],access.tools.map(t => [
    el('div',{},applicationName(t.connector),el('small',{},toolName(t.connector,t.tool))),
    el('div',{},badge(t.explicit_policy),t.requires_confirmation ? el('small',{},'Human approval always required') : null),
    el('div',{},badge(t.effective_policy),el('small',{},accessSummary(t)),t.resource_permissions_required?el('div',{},(t.resources||[]).map(r=>el('small',{},`${r.name} · resource ${r.mode}`)),el('a',{href:'#resources',onClick:()=>modal.close()},'Review folder permissions')):null)
  ])) : empty('No applications configured','Configure an application before granting access.'),
  el('div',{class:'form-actions'},button('Edit assigned capabilities',() => dialog('Assigned capabilities',[scopeChoices(catalog,agent.scopes)],'Save capabilities',async values => {
    await api(`/agents/${id}/scopes`,{method:'PATCH',body:{scopes:selectedScopes(values)}});modal.close();await refresh();
  }),'secondary'),button('Set tool permissions',() => {modal.close();location.hash='policies';},'secondary')));
  body.append(el('h3',{},'Connection'),el('p',{},'Each credential has its own scopes. Revoking one immediately blocks its new requests.'),
    catalog.connectors.some(c=>c.tools.some(t=>t.required_scopes.every(s=>agent.scopes.includes(s))))
      ? button('Connect Codex',() => {modal.close();connectAgent(agent,catalog,refresh);})
      : el('p',{class:'form-note'},'Assign capabilities for a configured application before connecting this agent.'));
  for (const c of agent.credentials) body.append(el('div',{class:'credential-row'},el('div',{},el('code',{},c.id.slice(0,12)),el('small',{},c.scopes.join(', ') || 'No capabilities')),badge(c.status),
    c.status==='active' ? button('Revoke',() => act(() => api(`/agents/${id}/credentials/${c.id}`,{method:'DELETE'})),'danger') : null));
  body.append(el('h3',{},'Agent controls'),el('div',{class:'form-actions'},['active','paused','disabled'].map(status => button(status==='active'?'Activate':status==='paused'?'Pause':'Disable',
    () => act(() => api(`/agents/${id}/status`,{method:'PATCH',body:{status}})),status==='disabled'?'danger':'secondary'))),button('Close',() => modal.close(),'secondary'));
  document.body.append(modal);modal.showModal();
}
