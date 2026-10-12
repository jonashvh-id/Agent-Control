import { api } from '../services/api.js';
import { el,heading,panel,empty,badge,date,table,button } from '../components/elements.js';
import { actorName,toolName,applicationName,explanation } from '../services/explanations.js';
export function agentObservation(agent,now=Date.now()) {
  if(agent.status!=='active')return agent.status;
  if(!agent.active_credentials)return 'no active credential';
  const seen=Date.parse(agent.last_request_at);
  return Number.isFinite(seen)&&seen<=now&&now-seen<600000?'request observed recently':'credential configured · no recent request';
}
export async function overview(actor,refresh) {
  if(actor.role!=='admin') {const requests=await api('/approvals');return el('div',{},heading('Review queue','Exact requests awaiting your decision.'),panel('Pending approvals',table(['Agent','Action','State'],requests.filter(a=>a.status==='pending').map(a=>[a.agent_name,toolName(a.connector,a.tool),badge(a.status)]))),button('Open approvals',()=>{location.hash='approvals';}));}
  const [data,settings]=await Promise.all([api('/operations'),api('/settings')]);const events=data.activity;
  const metrics=[['Completed actions · 24h',data.metrics.completed_24h],['Refused attempts · 24h',data.metrics.denied_attempts_24h],['Awaiting approval',data.metrics.pending_approvals],['Uncertain outcomes',data.metrics.uncertain_executions]];
  const connectionIssues=data.connections.filter(c=>c.connection.state!=='connected'||c.connection.native_verification?.verified!==true);
  const accesses=events.filter(e=>e.action==='execution.executed'&&e.objects?.length).slice(0,20);
  const refused=events.filter(e=>e.decision==='deny').slice(0,10);
  return el('div',{},heading('Operations','Current identities, resource access and decisions from your control plane.',button('Refresh',refresh,'secondary')),
    el('div',{class:'metrics'},metrics.map(([name,value])=>el('section',{class:'metric'},el('span',{},name),el('strong',{},value)))),
    panel('Intervention queue',el('div',{class:'prose'},
      settings.emergency_stop?el('p',{class:'error'},'Emergency stop is active. New actions are refused.'):null,
      !settings.execution_enabled?el('p',{class:'notice'},'Execution is disabled by server configuration.'):null,
      data.metrics.pending_approvals?el('p',{},`${data.metrics.pending_approvals} exact requests await review. `,el('a',{href:'#approvals'},'Open approvals')):null,
      data.metrics.uncertain_executions?el('p',{class:'error'},'Some provider outcomes are uncertain. Investigate their correlation IDs; do not blindly repeat writes.'):null,
      connectionIssues.map(c=>el('p',{},applicationName(c.connector),' · ',badge(c.connection.state),
        el('small',{},c.connection.native_verification?.reason?explanation(c.connection.native_verification.reason):'Configure or verify native access in Applications.'),el('a',{href:'#integrations'},'Review connection'))),
      !data.metrics.pending_approvals&&!data.metrics.uncertain_executions&&!connectionIssues.length&&!settings.emergency_stop?el('p',{},'No recorded intervention is pending.'):null)),
    panel('Agents and observed requests',data.agents.length?table(['Agent','Current state','Last request / completed action','Refusals · 24h'],data.agents.map(a=>[
      el('div',{},el('strong',{},a.name),el('small',{},a.runtime)),el('div',{},badge(a.status),el('small',{},agentObservation(a))),
      el('div',{},date(a.last_request_at),el('small',{},`Completed: ${date(a.last_used_at)}`)),a.denied_attempts_24h])):empty('No agents registered','Register one, assign tool/resource permissions, then issue a scoped credential.')),
    el('p',{class:'footnote'},data.presence),
    panel('Data consulted',accesses.length?table(['Who','Application / action','Folder','Data returned','Time'],accesses.map(e=>[
      actorName(e),el('div',{},applicationName(e.connector),el('small',{},toolName(e.connector,e.tool))),e.resource_name,
      e.objects.map(o=>el('div',{},el('strong',{},o.name),el('small',{},e.tool==='files.search'?'Metadata only':'Bounded document text'))),date(e.created_at)])):empty('No resource observations yet','Managed resource access records encrypted metadata for operator views. Legacy pilot events have no document observations; their contents cannot be reconstructed.')),
    panel('Recent refusals',refused.length?table(['Who','Action','Reason','Trace'],refused.map(e=>[actorName(e),toolName(e.connector,e.tool)||e.action,explanation(e.action),el('code',{},e.correlation_id)])):empty('No refusal recorded','Blocked attempts appear here with a reason and correlation ID.')),
    el('p',{class:'footnote'},'Stored audit contains trusted resource references and keyed object references. Metadata is encrypted separately; document text and credentials are excluded. Latest operator activity is bounded to 100 events.'));
}
