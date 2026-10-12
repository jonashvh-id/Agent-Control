import { api } from '../services/api.js';
import { el,heading,panel,table,badge,button,empty,date } from '../components/elements.js';
import { dialog } from '../components/dialog.js';
import { applicationName,toolName } from '../services/explanations.js';
export async function approvalsPage(refresh) {
  const approvals=await api('/approvals');const pending=approvals.filter(a => a.status==='pending');
  const review=a => dialog('Review this exact request',[
    el('dl',{},el('dt',{},'Requested by'),el('dd',{},a.agent_name || a.agent_id),
      el('dt',{},'Application / action'),el('dd',{},`${applicationName(a.connector)} · ${toolName(a.connector,a.tool)}`),
      el('dt',{},'Exact request'),el('dd',{},a.input_preview ? el('pre',{class:'approval-preview'},JSON.stringify(a.input_preview,null,2)) : 'No preview is available.'),
      el('dt',{},'Credential'),el('dd',{},a.credential_id),el('dt',{},'Expires'),el('dd',{},date(a.expires_at*1000))),
    el('label',{},el('span',{},'Your decision'),el('select',{name:'decision'},el('option',{value:'denied'},'Deny'),el('option',{value:'approved'},'Approve this exact request once'))),
    el('details',{},el('summary',{},'Request fingerprint'),el('code',{class:'hash'},a.input_hash)),
    el('p',{class:'form-note'},'Approving does not execute the action. The agent must submit the same request and credential. Current permissions are checked again.')
  ],'Record my decision',async values => {await api(`/approvals/${a.id}/decision`,{method:'POST',body:values});await refresh();});
  return el('div',{},heading('Approvals','See who is asking, what will happen, and the exact parameters before deciding.'),
    panel(`Waiting for you · ${pending.length}`,pending.length ? el('div',{class:'approval-cards'},pending.map(a => el('article',{class:'approval-card'},
      el('div',{},el('strong',{},a.agent_name || a.agent_id.slice(0,12)),el('small',{},`${applicationName(a.connector)} · ${toolName(a.connector,a.tool)}`)),
      a.input_preview ? el('pre',{class:'approval-preview'},JSON.stringify(a.input_preview,null,2)) : null,
      el('small',{},`Expires ${date(a.expires_at*1000)}`),button('Review request',() => review(a),'secondary')))) : empty('Nothing needs your approval','Requests appear here when an agent uses an action governed by ASK.')),
    panel('Recent decisions',table(['Agent','Action','Status','Requested'],approvals.filter(a => a.status!=='pending').map(a => [a.agent_name || a.agent_id.slice(0,12),toolName(a.connector,a.tool),badge(a.status),date(a.created_at)]))));
}
