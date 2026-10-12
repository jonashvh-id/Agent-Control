import { api } from '../services/api.js';
import { el,button } from '../components/elements.js';
import { dialog,showCredential } from '../components/dialog.js';
import { scopeChoices,selectedScopes } from '../components/scopes.js';
export function codexConfig(url) {
  const parsed = new URL(url);
  if (!['http:','https:'].includes(parsed.protocol) || parsed.username || parsed.password || parsed.search || parsed.hash
      || parsed.protocol === 'http:' && !['127.0.0.1','localhost','[::1]'].includes(parsed.hostname)) throw new Error('invalid_mcp_url');
  return `[mcp_servers.agent_control]\nurl = ${JSON.stringify(url)}\nbearer_token_env_var = "AGENT_CONTROL_AGENT_TOKEN"\nrequired = true\n`;
}
export async function connectAgent(agent, catalog, refresh) {
  const choices = {connectors:catalog.connectors.map(c => ({...c,tools:c.tools.filter(t => t.required_scopes.every(s => agent.scopes.includes(s)))}))};
  dialog('Connect Codex', [el('p',{},'Choose capabilities for this credential. They cannot exceed the agent’s assigned scopes.'),scopeChoices(choices,[],'Capabilities for this credential'),
    el('ol',{},el('li',{},'Save the credential outside the repository and set AGENT_CONTROL_AGENT_TOKEN in Codex’s environment.'),
      el('li',{},'Add this configuration to Codex, then start a new session.'),el('li',{},'Only currently permitted tools are discovered. Human decisions stay in this control plane.')),
    el('pre',{class:'connection-config'},codexConfig(catalog.mcp_url)),
    button('Copy MCP configuration',async () => { await navigator.clipboard.writeText(codexConfig(catalog.mcp_url)); },'secondary')
  ],'Create scoped credential',async values => {
    const scopes=selectedScopes(values);if (!scopes.length) throw new Error('Select at least one capability');
    const result=await api(`/agents/${agent.id}/credentials`,{method:'POST',body:{scopes}});
    await refresh();showCredential(result.bearer_token);
  });
}
