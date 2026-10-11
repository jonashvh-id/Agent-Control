import { api } from '../services/api.js';
import { el, heading, panel, button, badge } from '../components/elements.js';
import { dialog } from '../components/dialog.js';
export async function securityPage(refresh, actor) {
  const settings = await api('/settings');
  const toggle = () => dialog(settings.emergency_stop ? 'Resume gateway admission' : 'Activate emergency stop',[
    el('p',{},settings.emergency_stop ? 'This permits new requests to pass the remaining security checks.' : 'This blocks all new executions through the gateway. It cannot undo completed operations or cancel in-flight external work.')],
    settings.emergency_stop ? 'Resume admission' : 'Stop admission', async () => { await api('/security/emergency-stop',{method:'PUT',body:{stopped:!settings.emergency_stop}}); await refresh(); });
  return el('div',{},heading('Security','Deterministic controls, enforced by the backend.'),
    panel('Global emergency stop',el('div',{class:'security-control'},el('div',{},badge(settings.emergency_stop?'Stopped':'Inactive',settings.emergency_stop?'denied':'neutral'),
      el('h3',{},'Control gateway admission'),el('p',{},'Pause new operations across every registered connector. All other checks continue to apply on resume.')),
      actor.role==='admin' ? button(settings.emergency_stop?'Resume admission':'Activate stop',toggle,settings.emergency_stop?'secondary':'danger') : badge('Administrator control','neutral'))),
    panel('Identity controls',el('div',{class:'prose'},el('p',{},'Administrators can pause or disable individual agents and revoke credentials from the Agents page.'),
      el('p',{},'Agent credentials grant no human session or administrator access. No adapter runs without current scopes and an explicit policy.'),
      el('p',{},'Cancellation, secret vault integration and SSO/MFA are planned.'))));
}
