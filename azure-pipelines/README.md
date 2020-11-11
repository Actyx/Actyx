
The files in this directory configure how azure pipelines are being run on
our two CI boxes ci-0.actyx.net and ci-2.actyx.net

CI jobs run on these machines as user ubuntu. Tools and headers that are
needed for the build can be added by logging into both CI servers and
installing them using `sudo apt install`. Currently used tools that are not
installed during make prepare are: jq, protoc, libssl-dev, build-essential.

Each machine provides a number of azure pipelines workers, currently 12.
These can be accessed with the pool Native.

The workers are started as systemd services, configured in
`/etc/systemd/system/vsts-agent.*.service` The working directory is a
subdirectory of `/ubuntu/home`, e.g. `/ubunutu/home/agents/agent-0` Each job
gets a numbered directory in `_work/`, e.g.
`/home/ubuntu/agents/agent-0/_work/4` Source is checked out into a
subdirectory called s, e.g. `/home/ubuntu/agents/agent-0/_work/4/s` The
current working directory while exexuting jobs is this directory.

On startup, each agent can be configured by some files in the agent
directory, `/home/ubuntu/agents/agent-0/runsvc.sh`

Environment variables are configured in the azure pipeline GUI. Environment
variables marked as secret must be explicitly imported into jobs in the env
section. E.g. AWS_ACCESS_KEY_ID: $(SECRET_AWS_ACCESS_KEY_ID)

NOTE: We rely on the host to periodically do `docker login` both to DockerHub
and GitHub Packages instead of relying on Azure Pipelines to do it, since
having Pipelines do it introduces a race condition where, if a job fails, it
logs out from DockerHub. Since we're using host-based agents, this logs out
every single running pipeline, leading to random failures. All CI hosts have
a crontab that logs in to both every hour. See
https://github.com/Actyx/Cosmos/pull/5138.
