"""Adapted effective_mode from ZNode AgentCapabilityPolicyService.

Explicit policies never override scope checks. Unconfigured tools always deny.
"""

from dataclasses import replace

AGENT_CAPABILITY_MODES = ("deny", "ask", "allow")


class PolicyEngine:
    def effective_mode(self, *, tool, granted_scopes, explicit_mode=None):
        if not set(tool.required_scopes).issubset(set(granted_scopes)):
            return "deny"
        if explicit_mode not in AGENT_CAPABILITY_MODES:
            return "deny"
        if tool.requires_confirmation and explicit_mode == "allow":
            return "ask"
        return explicit_mode

    def effective_tool(self, *, tool, granted_scopes, explicit_mode=None):
        mode = self.effective_mode(tool=tool, granted_scopes=granted_scopes, explicit_mode=explicit_mode)
        return None if mode == "deny" else replace(tool, requires_confirmation=mode == "ask")
