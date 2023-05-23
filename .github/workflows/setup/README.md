# GitHub Actions Runner Setup

GitHub Actions Runner can only run once per user, to address that simply create and configure `N` users.

## Create and configure the agents

`./setup_agents.sh` usage:

```
sudo ./setup_agents.sh <ACCESS_TOKEN> <REPO_OWNER> [<N_AGENTS>]
```

- `ACCESS_TOKEN` is GitHub's Personal Access Token — [it requires the `repo` scope](https://docs.github.com/en/rest/actions/self-hosted-runners?apiVersion=2022-11-28#about-self-hosted-runners-in-github-actions).
    - For users: https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token
    - For organizations: https://docs.github.com/en/organizations/managing-programmatic-access-to-your-organization/setting-a-personal-access-token-policy-for-your-organization#restricting-access-by-personal-access-tokens-classic
- `REPO_OWNER` is the name of the owner of the repository.
- `N_AGENTS` is the number of agents to create and configure.

> Launching each agent is done manually

## Delete the agents

`clear_agents.sh` usage:

```
sudo ./clear_agents.sh <N_AGENTS>
```

## Development

To setup a development environment, you need to:

- Fork [`Actyx`](https://github.com/Actyx/Actyx)
- Change the setup scripts to target a different Linux user (i.e. `USERNAME="gha"` in `setup.sh`)
    - If you're running this in your own computer, this step is optional
- Run the setup scripts
