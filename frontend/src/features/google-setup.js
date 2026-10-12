import { api } from '../services/api.js';
import { el,button,panel,badge,errorText } from '../components/elements.js';
const scope='https://www.googleapis.com/auth/drive.readonly';
export function googleAuthorizationUrl(value,origin) {
  const url=new URL(value),redirect=new URL(url.searchParams.get('redirect_uri'));
  if(url.origin!=='https://accounts.google.com'||url.pathname!=='/o/oauth2/v2/auth'||url.username||url.password||url.hash
    ||url.searchParams.get('scope')!==scope||url.searchParams.get('code_challenge_method')!=='S256'
    ||!url.searchParams.get('code_challenge')||!url.searchParams.get('state')||redirect.origin!==origin
    ||redirect.pathname!=='/api/v1/connections/google-drive/callback'||redirect.search||redirect.hash||redirect.username||redirect.password) throw new Error('invalid_google_authorization_url');
  return url.href;
}
export function googleSetup(status) {
  const modal=el('dialog',{class:'wide-dialog',onClose:()=>modal.remove()},el('h2',{},'Configure Google OAuth once'),
    el('div',{class:'prose'},el('p',{},'An administrator connects the dedicated Google reader account from this dashboard. No access token or refresh token is copied manually.'),
      el('ol',{},el('li',{},'In your own Google Cloud project, enable Drive API and register a Web application OAuth client. For testing, authorize the dedicated reader account as a test user. ',
        el('a',{href:'https://developers.google.com/identity/protocols/oauth2/web-server',target:'_blank',rel:'noopener noreferrer'},'Google setup documentation')),
        el('li',{},'Register this exact redirect URI:',el('pre',{class:'connection-config'},status.redirect_uri),button('Copy redirect URI',()=>navigator.clipboard.writeText(status.redirect_uri),'secondary')),
        el('li',{},'The server operator sets AGENT_CONTROL_GOOGLE_OAUTH_CLIENT_ID and AGENT_CONTROL_GOOGLE_OAUTH_SECRET_FILE. Store the client secret in a private owned file, mode 0600. Remove the legacy token-file configuration and restart the server.'),
        el('li',{},'Share only the intended folders with the dedicated identity as Reader. Do not use a general administrator account or domain-wide delegation.'),
        el('li',{},'Connect that account here, select its reader folders in Resources, verify the native perimeter, then assign resource and tool permissions to agents.')),
      el('p',{class:'notice'},'Google offers no read-only OAuth scope restricted to three folder IDs. drive.readonly restricts operations; the dedicated account’s native sharing restricts private resources. The dashboard verifies a snapshot and every execution checks again.')),
    button('Close',()=>modal.close(),'secondary'));
  document.body.append(modal);modal.showModal();
}
export async function googleConnection(refresh) {
  const status=await api('/connections/google-drive'),notice=el('div',{id:'google-connection-notice','aria-live':'polite'});
  const act=async action=>{let message;try{message=await action();}catch(e){message=e.message;}await refresh();if(typeof message==='string')(document.getElementById('google-connection-notice')||notice).replaceChildren(errorText(new Error(message)));};
  const connect=()=>act(async()=>{const result=await api('/connections/google-drive/start',{method:'POST',body:{}});location.assign(googleAuthorizationUrl(result.authorization_url,location.origin));});
  const returned=new URL(location.href).searchParams.get('google');if(returned){notice.append(returned==='connected'?el('p',{class:'notice'},'Google connected. Select reader folders and verify native access before granting agent permissions.'):errorText(new Error('Google connection was refused. Start a new sign-in from this dashboard.')));const clean=new URL(location.href);clean.searchParams.delete('google');history.replaceState(null,'',clean.href);}
  const configured=status.state!=='oauth_not_configured',connected=status.state==='connected';
  return panel('Google account',el('div',{class:'prose'},notice,
    el('div',{class:'connection-state'},badge(status.state),status.account?.email?el('strong',{},status.account.email):null),
    el('p',{},connected?'Access tokens are renewed on the server. Native tokens are never given to the agent.':configured?'Connect the dedicated reader identity. Existing Agent Control tool and resource permissions still apply.':'Register your OAuth application once to enable dashboard sign-in.'),
    connected?el('p',{},status.native_verification?.verified?'The last native inventory check matched selected reader folders. Every dispatch checks again.':'Select reader folders and verify their native perimeter before any agent can use this connection.'):null,
    el('div',{class:'form-actions'},configured?button(connected?'Reconnect Google account':'Connect Google account',connect):button('Configure Google OAuth',()=>googleSetup(status),'secondary'),
      connected?button('Select folders',()=>{location.hash='resources';},'secondary'):null,
      connected?button('Disconnect Google',()=>act(async()=>{const result=await api('/connections/google-drive',{method:'DELETE'});if(!result.native_revocation_confirmed)return 'Local access removed. Google grant revocation was not confirmed; review the grant in your Google account.';}),'danger'):null),
    el('p',{class:'footnote'},'Use a dedicated identity with Reader sharing only. OAuth authorization does not itself encode a folder boundary.')));
}
