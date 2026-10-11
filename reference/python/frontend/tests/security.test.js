import test from 'node:test';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import { api, setCsrf } from '../src/services/api.js';
async function sources(directory) {
  const entries = await readdir(directory,{withFileTypes:true});
  return (await Promise.all(entries.map(async e => e.isDirectory() ? sources(`${directory}/${e.name}`) :
    e.name.endsWith('.js') ? [await readFile(`${directory}/${e.name}`,'utf8')] : []))).flat();
}
test('frontend avoids unsafe HTML sinks and persistent credential storage', async () => {
  for(const source of await sources(fileURLToPath(new URL('../src',import.meta.url)))) {
    assert.doesNotMatch(source,/\.innerHTML|insertAdjacentHTML|localStorage|sessionStorage|\beval\s*\(/);
  }
});
test('API requests carry same-origin credentials and CSRF, and omit absent bodies', async () => {
  const original = globalThis.fetch;
  let received;
  globalThis.fetch = async (url,options) => { received={url,options}; return {ok:true,status:200,json:async()=>({ok:true})}; };
  try {
    setCsrf('fixture-csrf');
    await api('/agents',{method:'POST',body:{name:'Agent'}});
    assert.equal(received.options.credentials,'same-origin');
    assert.equal(received.options.headers['X-CSRF-Token'],'fixture-csrf');
    assert.equal(received.options.headers['X-Control-Request'],'1');
    assert.equal(received.options.body,'{"name":"Agent"}');
    await api('/agents');
    assert.equal(received.options.body,undefined);
  } finally { globalThis.fetch=original;setCsrf(''); }
});
