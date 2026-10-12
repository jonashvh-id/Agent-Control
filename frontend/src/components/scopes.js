import { el } from './elements.js';
import { applicationName, toolName } from '../services/explanations.js';
export const selectedScopes = values => Object.entries(values).filter(([key,value]) => key.startsWith('scope:') && value === '1').map(([key]) => key.slice(6));
export function scopeChoices(catalog, selected = [], legend = 'Capabilities assigned to this identity') {
  const known = new Map();
  for (const c of catalog.connectors) for (const t of c.tools) for (const scope of t.required_scopes)
    known.set(scope, `${applicationName(c.id)} · ${toolName(c.id,t.name)}`);
  for (const scope of selected) if (!known.has(scope)) known.set(scope,scope);
  return el('fieldset',{class:'scope-choices'},el('legend',{},legend),
    known.size ? [...known].map(([scope,label]) => el('label',{class:'check-choice'},
      el('input',{type:'checkbox',name:`scope:${scope}`,value:'1',checked:selected.includes(scope)}),
      el('span',{},label,el('small',{},scope)))) : el('p',{},'Configure an application first to select its capabilities.'),
    el('p',{class:'form-note'},'Scopes set a maximum. A tool policy and a current scoped credential are also required.'));
}
