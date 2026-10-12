import test from 'node:test';
import assert from 'node:assert/strict';
import { googleAuthorizationUrl } from '../src/features/google-setup.js';
import { folderId } from '../src/features/resources.js';
import { agentObservation } from '../src/pages/overview.js';
const origin='http://127.0.0.1:8010';
const authorization=()=>{const u=new URL('https://accounts.google.com/o/oauth2/v2/auth');u.search=new URLSearchParams({scope:'https://www.googleapis.com/auth/drive.readonly',code_challenge_method:'S256',code_challenge:'fixture',state:'fixture',redirect_uri:origin+'/api/v1/connections/google-drive/callback'});return u;};
test('dashboard redirects only to the fixed Google readonly PKCE flow and its own callback',()=>{
  assert.equal(googleAuthorizationUrl(authorization().href,origin),authorization().href);
  for(const [key,value] of [['scope','https://www.googleapis.com/auth/drive'],['code_challenge_method','plain'],['state',''],['redirect_uri','https://evil.example/callback']]){const u=authorization();u.searchParams.set(key,value);assert.throws(()=>googleAuthorizationUrl(u.href,origin));}
  for(const host of ['evil.example','accounts.google.com.evil.example']){const u=authorization();u.hostname=host;assert.throws(()=>googleAuthorizationUrl(u.href,origin));}
});
test('folder input accepts exact native identifiers and known folder links only',()=>{
  const id='reader_folder_01';assert.equal(folderId(id),id);assert.equal(folderId('https://drive.google.com/drive/u/0/folders/'+id+'?usp=sharing'),id);
  for(const link of ['https://evil.example/drive/folders/'+id,'http://drive.google.com/drive/folders/'+id,'https://drive.google.com/file/d/'+id+'/view','https://user:secret@drive.google.com/drive/folders/'+id,'short'])assert.throws(()=>folderId(link));
});
test('stateless MCP presence does not turn configured credentials into an online agent',()=>{
  const now=Date.parse('2026-10-12T00:00:00Z');const a={status:'active',active_credentials:1};
  assert.match(agentObservation(a,now),/no recent request/);assert.equal(agentObservation({...a,active_credentials:0},now),'no active credential');
  assert.equal(agentObservation({...a,last_request_at:'2026-10-11T23:59:00Z'},now),'request observed recently');
  assert.equal(agentObservation({...a,status:'paused',last_request_at:'2026-10-11T23:59:00Z'},now),'paused');
  assert.match(agentObservation({...a,last_request_at:'2026-10-13T00:00:00Z'},now),/no recent request/);
});
