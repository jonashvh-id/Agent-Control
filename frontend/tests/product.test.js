import test from 'node:test';
import assert from 'node:assert/strict';
import { codexConfig } from '../src/features/connect.js';
import { accessSummary,explanation,toolName,actorName } from '../src/services/explanations.js';
import { selectedScopes } from '../src/components/scopes.js';
test('effective denial is not presented as access despite explicit allow',() => {
  const text=accessSummary({explicit_policy:'allow',effective_policy:'deny',reason:'scope_missing'});
  assert.match(text,/missing a required scope/);assert.doesNotMatch(text,/Available with/);
  assert.match(accessSummary({effective_policy:'ask',reason:'authorized'}),/exact request.*human approval/);
  assert.match(explanation('google_access_denied'),/Google refused.*token expiry/);
  assert.match(explanation('mcp.authentication'),/credential was revoked/);
});
test('Codex connection configuration contains an environment reference only',() => {
  const config=codexConfig('http://127.0.0.1:8010/mcp/');
  assert.match(config,/bearer_token_env_var = "AGENT_CONTROL_AGENT_TOKEN"/);
  assert.doesNotMatch(config,/Authorization|ac_agent_/);
  for (const url of ['file:///private','http://evil.example/mcp/','https://user:secret@example.com/mcp/','https://example.com/mcp/?token=secret'])
    assert.throws(() => codexConfig(url));
});
test('scope forms exclude unrelated form values and unchecked capabilities',() => {
  assert.deepEqual(selectedScopes({'name':'agent','runtime':'Codex','scope:issues:read':'1','scope:issues:write':'0'}),['issues:read']);
  assert.equal(toolName('google-drive','files.read'),'Read a document');
});
test('human administration is attributed to the operator rather than the target agent',() => {
  assert.equal(actorName({actor_kind:'human',human_name:'operator',agent_name:'Codex pilot'}),'operator');
  assert.equal(actorName({actor_kind:'agent',human_name:null,agent_name:'Codex pilot'}),'Codex pilot');
});
