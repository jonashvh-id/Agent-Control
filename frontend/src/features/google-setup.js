import { el,button } from '../components/elements.js';
export function googleSetup(configured) {
  const modal=el('dialog',{class:'wide-dialog',onClose:()=>modal.remove()},el('h2',{},'Connect Google Drive'),el('div',{class:'prose'},
    el('p',{},configured?'The Drive adapter is registered. Google validates its private credential on each request.':'Google access has not been configured for this server.'),
    el('p',{},'A governance approval authorizes an exact agent action. Google authorization separately gives this server permission to read the selected resource.'),
    el('ol',{},
      el('li',{},'Obtain a dedicated, short-lived Google access token. Prefer an identity that can read only the test documents. For a local pilot, ',
        el('a',{href:'https://developers.google.com/oauthplayground/',target:'_blank',rel:'noopener noreferrer'},'Google OAuth 2.0 Playground'),
        ' supports the drive.readonly scope with Online access. This Google scope can read all Drive files accessible to the account; the gateway separately restricts calls to its configured file IDs.'),
      el('li',{},'The server operator saves the token in a private, owned file (mode 0600), outside version control. Never paste Google tokens into chat, agent prompts, or this interface.'),
      el('li',{},'Set AGENT_CONTROL_GOOGLE_DRIVE_TOKEN_FILE and AGENT_CONTROL_GOOGLE_DRIVE_FILES, then restart this owned server. The allowlist contains exact Google Docs or plain-text file IDs.'),
      el('li',{},'Enable Google Drive in Applications, assign the agent’s capabilities, and set Search to Allow and Read to Ask. A read then appears in Approvals before execution.')),
    el('p',{class:'notice'},'This pilot uses short-lived credentials. Automatic Google sign-in and token refresh in Agent Control are not available yet. Expired or revoked native access is refused.')),
    button('Close',()=>modal.close(),'secondary'));
  document.body.append(modal);modal.showModal();
}
