import { api } from '../services/api.js';
import { el, heading, panel, table, badge, button, empty, date } from '../components/elements.js';
import { dialog } from '../components/dialog.js';
export async function approvalsPage(refresh) {
  const approvals = await api('/approvals');
  const review = approval => dialog('Review immutable request',[
    el('dl',{},el('dt',{},'Agent'),el('dd',{},approval.agent_id),el('dt',{},'Target operation'),el('dd',{},`${approval.connector} / ${approval.tool}`),
      el('dt',{},'Exact request preview'),el('dd',{},approval.input_preview ? el('pre',{class:'approval-preview'},JSON.stringify(approval.input_preview,null,2)) : `${approval.input_summary.parameter_count} parameters. No connector preview is available.`),
      el('dt',{},'Request fingerprint'),el('dd',{class:'hash'},approval.input_hash),el('dt',{},'Expires'),el('dd',{},date(approval.expires_at*1000))),
    el('label',{},el('span',{},'Decision'),el('select',{name:'decision'},el('option',{value:'denied'},'Deny'),el('option',{value:'approved'},'Approve'))),
    el('p',{class:'form-note'},'Approval binds the exact request and credential. Current authorization is checked again before execution.')],
    'Record decision',async values => { await api(`/approvals/${approval.id}/decision`,{method:'POST',body:values}); await refresh(); });
  return el('div',{},heading('Approvals','Human decisions for sensitive operations. Each grant is bound to one request.'),
    panel('Request queue & decisions',approvals.length ? table(['Agent','Operation','Status','Requested',''],approvals.map(a => [a.agent_id.slice(0,12),`${a.connector} / ${a.tool}`,badge(a.status),date(a.created_at),
      a.status==='pending' ? button('Review',()=>review(a),'secondary') : el('span',{class:'muted'},a.decided_by ? 'Decision recorded' : 'Closed')])) : empty('No requests awaiting review','Approval requests appear when a registered tool has an ASK policy. Requests must pass the same authorization checks before execution.')));
}
