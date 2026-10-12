import { api } from '../services/api.js';
import { el, heading, button, panel, table, badge, empty } from '../components/elements.js';
import { dialog, input, select } from '../components/dialog.js';
export async function policiesPage(refresh, actor) {
  const policies = await api('/policies');
  const agents = actor.role === 'admin' ? await api('/agents') : [];
  const edit = () => dialog('Set a tool policy', [select('Agent','agent_id',agents.map(a => [a.id,a.name])),
    input('Connector identifier','connector',{placeholder:'github'}), input('Tool identifier','tool',{placeholder:'issues.create'}),
    select('Decision','mode',[['deny','DENY · Prohibited'],['ask','ASK · Human approval'],['allow','ALLOW · Subject to security checks']]),
    el('p',{class:'form-note'},'Policies can be prepared for future tools. A policy does not register or enable an adapter.')], 'Save policy', async values => {
      await api('/policies',{method:'PUT',body:values}); await refresh();
    });
  return el('div', {},heading('Policies', 'Deny by default. Allow only the capabilities you choose.',
    actor.role === 'admin' ? button('＋ Set policy',edit,agents.length ? 'primary' : 'secondary') : null),
    el('p',{class:'notice'},agents.length || actor.role !== 'admin' ? 'Scope checks and mandatory confirmation always apply before execution.' : 'Register an agent before setting a policy.'),
    panel('Explicit tool policies',policies.length ? table(['Agent','Connector','Tool','Decision','Updated'],policies.map(p => [agents.find(a => a.id===p.agent_id)?.name || p.agent_id.slice(0,12),p.connector,p.tool,badge(p.mode),new Date(p.updated_at).toLocaleString()])) : empty('Default deny is active','No explicit policies have been configured.')));
}
