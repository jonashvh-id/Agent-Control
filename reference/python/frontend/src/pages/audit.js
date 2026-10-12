import { api } from '../services/api.js';
import { el, heading, panel, table, badge, empty, date } from '../components/elements.js';
export async function auditPage() {
  const events = await api('/audit');
  return el('div',{},heading('Activity & audit','Actual governance events, with sensitive request values omitted.'),
    panel('Event ledger · latest 100',events.length ? table(['Time','Actor','Operation','Decision','Correlation'],events.map(e => [date(e.created_at),
      el('div',{},e.actor_kind,el('small',{},e.actor_id.slice(0,12) || 'Unauthenticated')),el('div',{},e.action,el('small',{},e.tool || 'Control plane')),badge(e.decision),el('code',{},e.correlation_id.slice(0,12))])) : empty('No audit events','Events are recorded as administration and gateway requests occur.')),
    el('p',{class:'footnote'},'Append-only application ledger. External tamper-evident storage and retention controls are planned.'));
}
