"""Opt-in model-driven MCP pilot. Test tooling only; no application Python dependency."""
import argparse
import json
import os
from pathlib import Path
import stat
import subprocess
import urllib.parse


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--origin', required=True)
    parser.add_argument('--token-file', type=Path, required=True)
    parser.add_argument('--out-dir', type=Path, required=True)
    parser.add_argument('--prompt', required=True)
    args = parser.parse_args()
    origin = urllib.parse.urlsplit(args.origin)
    if origin.scheme != 'http' or origin.hostname not in ('127.0.0.1', 'localhost') or origin.username or origin.password or origin.query or origin.fragment or origin.path:
        raise SystemExit('Pilot requires a bare loopback HTTP origin.')
    fd = os.open(args.token_file, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, 'r') as handle:
        meta = os.fstat(handle.fileno())
        if not stat.S_ISREG(meta.st_mode) or meta.st_uid != os.geteuid() or meta.st_mode & 0o077 or meta.st_size > 256:
            raise SystemExit('A private owned agent credential file is required.')
        token = handle.read(257).strip()
    if not token.startswith('ac_agent_') or len(token) > 256:
        raise SystemExit('Invalid pilot agent credential.')
    args.out_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
    os.chmod(args.out_dir, 0o700)
    workspace = args.out_dir / 'workspace'
    workspace.mkdir(mode=0o700, exist_ok=True)
    command = ['codex', 'exec', '--ignore-user-config', '--ephemeral', '--json',
               '--skip-git-repo-check', '--sandbox', 'read-only', '--cd', str(workspace)]
    for feature in ('shell_tool', 'unified_exec', 'apps', 'plugins', 'browser_use', 'browser_use_external',
                    'in_app_browser', 'computer_use', 'image_generation', 'multi_agent', 'workspace_dependencies', 'hooks'):
        command += ['--disable', feature]
    config = {
        'mcp_servers.agent_control.url': args.origin + '/mcp/',
        'mcp_servers.agent_control.bearer_token_env_var': 'AGENT_CONTROL_AGENT_TOKEN',
        'mcp_servers.agent_control.required': True,
        'mcp_servers.agent_control.startup_timeout_sec': 20,
        'mcp_servers.agent_control.tool_timeout_sec': 25,
        'mcp_servers.agent_control.default_tools_approval_mode': 'approve',
    }
    for key, value in config.items():
        command += ['-c', key + '=' + json.dumps(value)]
    command += ['-']
    # Preserve the existing account's auth location; do not forward unrelated secret variables.
    environment = {key: os.environ[key] for key in ('PATH', 'HOME', 'CODEX_HOME', 'XDG_CONFIG_HOME', 'LANG') if key in os.environ}
    environment['AGENT_CONTROL_AGENT_TOKEN'] = token
    result = subprocess.run(command, input=args.prompt, text=True, capture_output=True,
                            env=environment, timeout=180)
    for name, data in [('events.jsonl', result.stdout), ('diagnostics.txt', result.stderr)]:
        path = args.out_dir / name
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, 'w') as handle:
            handle.write(data.replace(token, '[REDACTED_AGENT_TOKEN]'))
    calls = []
    for line in result.stdout.splitlines():
        try:
            event = json.loads(line)
        except ValueError:
            continue
        item = event.get('item', {})
        if event.get('type') == 'item.completed' and item.get('type') == 'mcp_tool_call':
            call = {'server': item.get('server'), 'tool': item.get('tool'), 'status': item.get('status'), 'error': bool(item.get('error'))}
            for content in (item.get('result') or {}).get('content', []):
                if content.get('type') == 'text':
                    try:
                        body = json.loads(content['text'])
                    except (ValueError, KeyError):
                        continue
                    if isinstance(body, dict):
                        call.update({key: body[key] for key in ('http_status', 'status', 'execution_state') if key in body})
                        call['error'] = call['error'] or bool(body.get('error'))
                        if body.get('error'):
                            call['reason_code'] = body['error']
            calls.append(call)
    summary = {'client': 'Codex CLI', 'model_generated_mcp_calls': calls,
               'exit_code': result.returncode, 'shell_and_other_apps_disabled': True,
               'ephemeral_session': True,
               'required_mcp_authentication_refused': 'required MCP servers failed to initialize' in result.stderr and 'Auth required' in result.stderr}
    summary_path = args.out_dir / 'summary.json'
    fd = os.open(summary_path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'w') as handle:
        handle.write(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary))
    raise SystemExit(result.returncode)


if __name__ == '__main__':
    main()
