import { api } from '../services/api.js';
import { el, heading, panel, empty, badge, table } from '../components/elements.js';
export async function integrationsPage() {
  const connections = await api('/integrations');
  return el('div',{},heading('Integrations','Trusted adapters connect your gateway to external applications.'),
    panel('Registered connectors',connections.length ? table(['Connector','State'],connections.map(c => [c.id,badge(c.enabled?'Enabled':'Disabled')])) : empty('No external connectors configured','The GitHub pilot requires explicit configuration. Google Workspace and Microsoft 365 adapters remain planned.')),
    el('div',{class:'foundation-banner'},el('div',{},el('strong',{},'An explicit boundary'),el('p',{},'Agent Control governs connections routed through this gateway. Direct connections to external applications remain outside its control.')),badge('Planned','foundation')));
}
