import { api } from '../services/api.js';
import { el,heading,panel,empty,badge,table,button,errorText } from '../components/elements.js';
import { applicationName,toolName } from '../services/explanations.js';
import { googleConnection } from '../features/google-setup.js';
export async function integrationsPage(refresh,actor) {
  const catalog=await api('/catalog');const notice=el('div',{'aria-live':'polite'});
  const change=async c => {try {await api(`/integrations/${c.id}`,{method:'PATCH',body:{enabled:!c.enabled}});await refresh();} catch(e) {notice.replaceChildren(errorText(e));}};
  return el('div',{},heading('Applications','Only configured applications can expose governed actions.'),notice,
    panel('Configured applications',catalog.connectors.length ? table(['Application','Available actions','State',''],catalog.connectors.map(c => [
      applicationName(c.id),c.tools.map(t => el('span',{class:'capability'},toolName(c.id,t.name))),badge(c.enabled?'enabled':'disabled'),
      actor.role==='admin'?button(c.enabled?'Disable':'Enable',() => change(c),'secondary'):null
    ])) : empty('Configure your first application','GitHub and Google Drive are available adapters. Configure a dedicated credential and resource allowlist, then enable the application here.')),
    el('p',{class:'notice'},catalog.execution_enabled?'Tool policies and native permissions apply to every request.':'Execution is currently disabled for this instance.'),
    actor.role==='admin'?await googleConnection(refresh):null,
    el('p',{class:'footnote'},'Direct application connections outside this gateway are outside its control.'));
}
