import { api } from '../services/api.js';
import { el,heading,panel,table,badge,button,empty,errorText } from '../components/elements.js';
import { dialog,input,select } from '../components/dialog.js';
export function folderId(value) {
  if(/^[A-Za-z0-9_-]{10,128}$/.test(value))return value;
  const url=new URL(value);const match=url.pathname.match(/^\/drive\/(?:u\/\d+\/)?folders\/([A-Za-z0-9_-]{10,128})\/?$/);
  if(url.origin!=='https://drive.google.com'||url.username||url.password||!match)throw new Error('Use a Google Drive folder link or exact folder ID');
  return match[1];
}
export async function resourcesPage(refresh,actor) {
  if(actor.role!=='admin')return el('div',{},heading('Resources','Resource configuration requires an administrator.'));
  const [roots,policies,agents,status]=await Promise.all([api('/resources'),api('/resource-policies'),api('/agents'),api('/connections/google-drive')]);
  const notice=el('div',{id:'resource-notice','aria-live':'polite'});const connected=status.state==='connected';
  const act=async fn=>{let message;try{message=await fn();}catch(e){message=e.message;}await refresh();if(typeof message==='string')(document.getElementById('resource-notice')||notice).replaceChildren(errorText(new Error(message)));};
  const choose=async()=>{try {
    const result=await api('/connections/google-drive/folders');const choices=result.folders.filter(f=>f.reader_only&&!roots.some(r=>r.provider_id===f.provider_id));
    const candidate=choices.length?select('Folders accessible to the dedicated identity','candidate',[['','Choose a folder'],...choices.map(f=>[f.provider_id,f.name])]):null;
    if(candidate)candidate.querySelector('select').required=false;
    dialog('Add a reader folder',[
      candidate||el('p',{},'No new reader folder appears in this list. Check native sharing or use its exact link.'),
      input('Or paste its folder link / ID','folder',{required:false}),
      el('p',{class:'form-note'},'The server checks the folder’s native permissions. Adding it grants no agent access. Native verification is required after changing the set.')
    ],'Add folder',async values=>{const provider_id=folderId(values.folder||values.candidate);await api('/resources',{method:'POST',body:{connector:'google-drive',provider_id}});await refresh();});
  }catch(e){notice.replaceChildren(errorText(e));}};
  const grant=(preset={})=>{
    const a=select('Agent','agent_id',agents.map(a=>[a.id,a.name]));const r=select('Folder','resource_id',roots.filter(r=>r.enabled||r.id===preset.resource_id).map(r=>[r.id,r.name+(r.enabled?'':' (disabled)')]));
    const m=select('Resource permission','mode',[['deny','Blocked'],['ask','Ask for every exact action in this folder'],['allow','Allow within the tool’s permission']]);
    if(preset.agent_id)a.querySelector('select').value=preset.agent_id;if(preset.resource_id)r.querySelector('select').value=preset.resource_id;if(preset.mode)m.querySelector('select').value=preset.mode;
    dialog('Choose resource access',[a,r,m,el('p',{class:'form-note'},'Resource and tool permissions combine with the stricter rule. Scopes, current credentials and native Google permissions also apply.')],'Save resource permission',async values=>{await api('/resource-policies',{method:'PUT',body:values});await refresh();});
  };
  return el('div',{},heading('Resources','Choose native reader folders, then decide which agents can use them.',connected?button('＋ Add reader folder',choose):null),notice,
    !connected?empty('Connect the dedicated Google account','Open Applications and configure the integrated OAuth connection.'):null,
    connected?panel('Native perimeter',el('div',{class:'prose'},badge(status.native_verification?.verified?'verified snapshot':'verification required'),
      el('p',{},'The identity must have native Reader access only to selected folders. Extra visible objects or incomplete checks block agent actions. Moving a document outside its allowed folder blocks its content read.'),
      status.native_verification?.outside_objects?el('p',{class:'error'},`${status.native_verification.outside_objects} visible objects are outside the selected folders. Remove those native shares or select the intended reader folders.`):null,
      button('Verify native access',()=>act(async()=>{const result=await api('/connections/google-drive/verify',{method:'POST',body:{}});if(!result.verified)throw new Error('Google account has access outside selected folders. Reduce native sharing before using it.');}),'secondary'))):null,
    panel('Selected folders',roots.length?table(['Folder','Platform state',''],roots.map(r=>[el('div',{},el('strong',{},r.name),el('small',{},r.provider_id)),badge(r.enabled?'enabled':'disabled'),
      button(r.enabled?'Disable in Agent Control':'Enable',()=>act(()=>api(`/resources/${r.id}`,{method:'PATCH',body:{enabled:!r.enabled}})),'secondary')])):empty('No folders selected','Add the folders shared with the dedicated reader identity. Every agent starts with DENY.')),
    panel('Agent access',el('div',{},agents.length&&roots.some(r=>r.enabled)?button('＋ Set resource permission',()=>grant(),'secondary'):null,
      policies.length?table(['Agent','Folder','Permission',''],policies.map(p=>[agents.find(a=>a.id===p.agent_id)?.name||p.agent_id,roots.find(r=>r.id===p.resource_id)?.name||p.resource_id,badge(p.mode),button('Edit',()=>grant(p),'text')])):empty('No resource access granted','Tool ALLOW alone cannot grant access to a managed Drive folder.'))),
    el('p',{class:'footnote'},'Disabling a resource here does not remove its Google sharing. The read-only OAuth scope cannot change native ACLs; their owner must remove those grants in Google.'));
}
