import { api } from '../services/api.js';
import { el,heading,button,panel,table,badge,empty } from '../components/elements.js';
import { dialog,select } from '../components/dialog.js';
import { applicationName,toolName } from '../services/explanations.js';
export async function policiesPage(refresh,actor) {
  const [policies,catalog,agents]=await Promise.all([api('/policies'),api('/catalog'),actor.role==='admin'?api('/agents'):Promise.resolve([])]);
  const tools=catalog.connectors.flatMap(c => c.tools.map(t => ({...t,connector:c.id})));
  const edit=(preset={}) => {
    const agentSelect=select('Agent','agent_id',agents.map(a => [a.id,a.name]));
    const toolSelect=select('Application / action','capability',tools.map(t => [`${t.connector}/${t.name}`,`${applicationName(t.connector)} · ${toolName(t.connector,t.name)}`]));
    const mode=select('Permission','mode',[['deny','Blocked'],['ask','Ask me for each exact request'],['allow','Allow within assigned scopes']]);
    const note=el('p',{class:'form-note'});
    const update=() => {const t=tools.find(t => `${t.connector}/${t.name}`===toolSelect.querySelector('select').value);
      note.textContent=t?.requires_confirmation?'This action always requires human approval. Choosing Allow still results in ASK.':'A policy cannot exceed assigned scopes, active credential scopes or native application permissions.';};
    toolSelect.addEventListener('change',update);
    if(preset.agent_id) agentSelect.querySelector('select').value=preset.agent_id;
    if(preset.tool) toolSelect.querySelector('select').value=`${preset.connector}/${preset.tool}`;
    if(preset.mode) mode.querySelector('select').value=preset.mode;update();
    dialog('Choose an agent permission',[agentSelect,toolSelect,mode,note],'Save permission',async values => {
      const [connector,tool]=values.capability.split('/');
      await api('/policies',{method:'PUT',body:{agent_id:values.agent_id,connector,tool,mode:values.mode}});await refresh();
    });
  };
  return el('div',{},heading('Permissions','Every action starts blocked. Grant only what each agent needs.',
    actor.role==='admin' && agents.length && tools.length ? button('＋ Set permission',() => edit()) : null),
    el('p',{class:'notice'},!tools.length?'Configure an application to choose its registered actions.':!agents.length && actor.role==='admin'?'Register an agent first.':'The access view on each agent shows whether this permission is currently usable.'),
    panel('Tool permissions',policies.length ? table(['Agent','Application / action','Permission',''],policies.map(p => [agents.find(a => a.id===p.agent_id)?.name || p.agent_id.slice(0,12),
      el('div',{},applicationName(p.connector),el('small',{},toolName(p.connector,p.tool))),badge(p.mode),
      actor.role==='admin' && tools.some(t => t.name===p.tool && t.connector===p.connector)?button('Edit',() => edit(p),'text'):null])) : empty('All actions are blocked','Choose an agent and a registered action to grant permission.')));
}
