import { api } from '../services/api.js';
import { el, heading, panel, table, badge } from '../components/elements.js';
export async function settingsPage() {
  const settings = await api('/settings');
  return el('div',{},heading('Settings','Instance configuration and foundation capabilities.'),
    panel('Instance',table(['Configuration','Value'],[['Version',settings.version],['Stage',badge(settings.stage,'foundation')],['Default policy',badge(settings.default_policy)],
      ['Execution enabled',String(settings.execution_enabled)],['Registered tools',String(settings.registered_tools)],['MCP transport',badge(settings.mcp_transport,'neutral')]])),
    panel('Development status',el('div',{class:'prose'},el('p',{},'Working: human authentication, agent registry, scoped credentials, explicit policies, approval decisions, audit and emergency stop.'),
      el('p',{},'Planned: additional external adapters, OAuth, managed credential vault, credential expiry, SSO/MFA, agent health monitoring and long-running operation cancellation.'),
      el('p',{},'Runtime configuration is managed through the process environment. This release has no production deployment or security-readiness certification.'))));
}
