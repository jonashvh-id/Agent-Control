from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field

Scope = Annotated[str, Field(pattern=r"^[a-z][a-z0-9_.-]{0,63}:[a-z][a-z0-9_-]{0,31}$")]

Identifier = r"^[a-z][a-z0-9_.:-]{0,127}$"


class StrictModel(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)


class Login(StrictModel):
    username: str = Field(min_length=1, max_length=80, pattern=r"^[A-Za-z0-9_.@-]+$")
    password: str = Field(min_length=1, max_length=256)


class AgentCreate(StrictModel):
    name: str = Field(min_length=1, max_length=80)
    description: str = Field(default="", max_length=500)
    provider: str = Field(default="", max_length=40)
    runtime: str = Field(default="", max_length=80)
    scopes: list[Scope] = Field(default_factory=list, max_length=100)


class CredentialCreate(StrictModel):
    scopes: list[Scope] = Field(default_factory=list, max_length=100)


class AgentStatus(StrictModel):
    status: Literal["active", "paused", "disabled"]


class PolicySet(StrictModel):
    agent_id: str = Field(pattern=r"^[a-f0-9]{32}$")
    connector: str = Field(pattern=r"^[a-z][a-z0-9_-]{0,63}$")
    tool: str = Field(pattern=Identifier)
    mode: Literal["deny", "ask", "allow"]


class ApprovalDecision(StrictModel):
    decision: Literal["approved", "denied"]


class StopSet(StrictModel):
    stopped: bool


class EnabledSet(StrictModel):
    enabled: bool
