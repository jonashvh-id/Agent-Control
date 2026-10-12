import { api } from '../services/api.js';
import { el, heading, panel, empty, badge, date, table } from '../components/elements.js';
export async function overview(actor) {
  const [settings, approvals, audit, integrations] = await Promise.all([api('/settings'),api('/approvals'),api('/audit'),api('/integrations')]);
  const agents = actor.role === 'admin' ? await api('/agents') : null;
  const metrics = [['Registered agents', agents?.length ?? 'Admin view', 'Identities in this instance'],
    ['Pending approvals', approvals.filter(a => a.status === 'pending').length, 'Awaiting human review'],
    ['Configured applications', integrations.length, 'Trusted execution adapters'],
    ['Default policy', 'Deny', 'Explicit permission required']];
  return el('div', {}, heading('Govern with confidence.', 'A clear view of your agent identities, decisions and execution boundaries.'),
    el('div', { class: 'foundation-banner' }, el('div', {}, el('strong', {}, 'Your next steps: configure applications, register an agent, and choose its permissions.'),
      el('p', {}, 'Connect Codex with a scoped credential. Requests governed by ASK appear in Approvals; blocked actions are explained in Activity & audit.')), badge('Product pilot', 'foundation')),
    el('div', { class: 'metrics' }, metrics.map(([label,value,description]) => el('section', { class: 'metric' }, el('span', {}, label), el('strong', {}, value), el('small', {}, description)))),
    el('div', { class: 'overview-grid' }, panel('Recent activity', audit.length ? table(['Operation','Decision','Time'], audit.slice(0,6).map(e => [e.action,badge(e.decision),date(e.created_at)])) : empty('No activity recorded', 'Administration and gateway events will appear here.')),
      panel('Execution boundary', el('div', { class: 'boundary' }, el('div', {}, 'External agent', el('small', {}, 'Untrusted proposals')), el('span', {}, '↓'),
        el('div', { class: 'boundary-gateway' }, 'Agent Control', el('small', {}, 'Identity → Policy → Approval → Audit')), el('span', {}, '↓'),
        el('div', {}, 'Trusted connector', el('small', {}, integrations.length ? `${integrations.length} registered` : 'No external adapters configured'))))),
    panel('Control state', el('div', { class: 'status-list' }, el('div', {}, 'Emergency stop', badge(settings.emergency_stop ? 'Stopped' : 'Inactive', settings.emergency_stop ? 'denied' : 'neutral')),
      el('div', {}, 'Execution', badge(settings.execution_enabled ? 'Enabled by configuration' : 'Disabled', 'neutral')),
      el('div', {}, 'Coverage', el('span', {}, settings.coverage)))));
}
