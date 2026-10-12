import { api } from '../services/api.js';
import { el,heading,panel,table,badge,empty,date } from '../components/elements.js';
import { applicationName,toolName,explanation,actorName } from '../services/explanations.js';
export async function auditPage() {
  const events=await api('/audit');
  return el('div',{},heading('Activity & audit','Understand each decision. Sensitive request contents stay out of this ledger.'),
    panel('Latest 100 events',events.length ? table(['Time','Who','Action','Decision / explanation','Trace'],events.map(e => [date(e.created_at),
      el('div',{},actorName(e),el('small',{},`${e.actor_kind} · ${e.actor_id.slice(0,12)}`)),
      el('div',{},e.tool?toolName(e.connector,e.tool):e.action.replaceAll('.',' '),el('small',{},e.connector?applicationName(e.connector):'Control plane'),
        e.actor_kind==='human' && e.agent_name ? el('small',{},`For ${e.agent_name}`) : null),
      el('div',{},badge(e.decision),el('small',{},e.decision==='deny' || e.decision==='denied'?explanation(e.action):e.action.replaceAll('.',' '))),el('code',{},e.correlation_id)
    ])) : empty('No events yet','Agent requests and administrative changes appear here.')),
    el('p',{class:'footnote'},'Correlation IDs connect requests, approvals and execution. External tamper-evident storage and retention controls remain planned.'));
}
