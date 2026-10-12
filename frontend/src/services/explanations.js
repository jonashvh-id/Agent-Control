export const applicationName = id => ({github:'GitHub','google-drive':'Google Drive'}[id] || id);
export const toolName = (connector, tool) => ({'github:issues.list':'Read issues','github:issues.create':'Create an issue',
  'google-drive:files.search':'Find documents','google-drive:files.read':'Read a document'}[`${connector}:${tool}`] || tool);
export const actorName = event => event.actor_kind === 'human' ? event.human_name || 'Human operator' : event.agent_name || 'Unknown requester';
const reasons = {
  authorized:'Available with a current credential. Native application permissions still apply.',
  default_deny:'No permission has been set for this tool. Choose a policy to grant access.',
  policy_denied:'The policy or credential scopes do not allow this action. Check the agent’s access.',
  scope_missing:'An active credential is missing a required scope. Review assigned scopes and issue a matching credential.',
  no_active_credential:'Assign capabilities and a tool permission, then connect with a scoped credential.',
  agent_inactive:'This agent is paused or disabled. New requests are blocked.',
  agent_authentication_required:'The agent credential is missing, invalid or revoked.',
  'mcp.authentication':'MCP authentication was refused. Check whether the credential was revoked or the agent is paused or disabled.',
  'gateway.authentication':'Gateway authentication was refused. Check the credential and the agent’s current state.',
  connector_disabled:'This application is disabled. An administrator must enable it.',
  execution_disabled:'Execution is disabled for this instance. The operator must enable it.',
  emergency_stop:'Emergency stop is active. New actions are blocked for every agent.',
  approval_binding_mismatch:'The request differs from the one approved. Submit the original request or request a new approval.',
  approval_unusable:'The approval may be expired, denied or already consumed.',
  approval_authority_expired:'The approval expired or its reviewer is no longer authorized.',
  approval_required_after_policy_change:'The policy now requires approval. Submit a new request.',
  idempotency_conflict:'This key was used for different parameters. Do not retry a possibly dispatched write.',
  github_access_denied:'GitHub refused access. Check the native credential and repository permissions.',
  google_access_denied:'Google refused access. Check token expiry and the document’s native permissions.',
  google_file_denied:'This document is outside the configured file allowlist.',
  google_file_type_unsupported:'This pilot reads Google Docs and plain-text files only.',
  google_credential_unavailable:'The operator must configure or rotate the dedicated Google credential.',
  tool_not_registered:'This tool is not registered. A policy alone does not install an application.',
  connector_timeout:'The provider did not finish in time. Its outcome may be uncertain; a dispatched write must not be retried.',
};
export const explanation = code => reasons[code] || 'Recorded control-plane event. Use the correlation ID when investigating.';
export function accessSummary(item) {
  if (item.effective_policy === 'deny') return explanation(item.reason);
  return item.effective_policy === 'ask' ? 'Every exact request needs a separate human approval.' : explanation('authorized');
}
