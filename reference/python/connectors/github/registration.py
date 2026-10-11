from .adapter import GitHubAdapter
from .tools import definitions


def register_github(registry, config):
    adapter = GitHubAdapter(config)
    for definition in definitions(config):
        registry.register("github", definition, adapter)
