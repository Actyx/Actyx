# Setup of EC2 Windows nodes with ActyxOS

## Getting Started

Ansible is a Python tool, so you’ll need to have Python 3.7 installed (if you have a different version, you may try to edit Pipfile).
The first step is to `pip install --user pipenv` (might be called `pip3` depending on distro) and then `pipenv install` the project setup.
The playbook needs some environment variables as input and is started like so:

```bash
export CI_RUN="<some unique identifier for this run>"
export NODE_NAME="<whatever you want in the Name tag; node will be reused if existing is found>"
export SSH_PUBLIC_KEY="<filename of the public key to use for SSH access>"
export EC2_ADMIN_PW="<some random string, only used internally>"
export EC2_KEY_NAME="<name of the keypair; not used, but must exist>"
export EC2_IMAGE_ID="<some Windows AMI>"
export EC2_INSTANCE_TYPE="<instance type>"

pipenv run ansible-playbook -i inventory/actyx.aws_ec2.yml -v playbook.yml
```

## How it works

The playbook consists of two steps:

- first we operate on `localhost`, starting EC2 instances (see `roles/run_ec2/tasks/main.yml` for the procedure)
- second we operate on all hosts previously started, installing OpenSSH and The Package (see `roles/prepare_windows/tasks.main.yml`)

The Yaml task definitions reference Ansible modules, see [the list](https://docs.ansible.com/ansible/2.8/modules/list_of_all_modules.html).
As an example:

```yaml
- name: what am I doing
  file:
    path: user_data
    state: directory
```

This defines a process step named “what am I doing” that uses the [file](https://docs.ansible.com/ansible/2.8/modules/file_module.html#file-module) module to ensure that some directory exists.
Variables are referenced with template expressions like `{{ my_variable }}` that usually need to be quoted (a Yaml value starting with `{` is parsed as JSON object).
Variables get their defaults in the role’s `defaults/main.yml` file, overridden from the playbook.

The set of hosts that is used in the second step is the `windows` group which is defined in the `inventory/actyx.aws_ec2.yml` file.
This inventory plugin is particularly badly documented, so you’ll need to read [the source](https://github.com/alibaba/ansible-provider-docs/blob/master/lib/ansible/plugins/inventory/aws_ec2.py#L466) to figure things out.
