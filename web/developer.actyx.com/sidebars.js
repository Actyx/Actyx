module.exports = {
  howToSidebar: [
    'how-to/overview',
    {
      type: 'category',
      label: 'Local development',
      collapsed: true,
      items: [
        'how-to/local-development/install-actyx',
        'how-to/local-development/install-cli-node-manager',
        'how-to/local-development/add-node-node-manager',
        'how-to/local-development/reset-your-node',
      ],
    },
    {
      type: 'category',
      label: 'Business logic',
      collapsed: true,
      items: [
        'how-to/business-logic/tracking-state',
        'how-to/business-logic/committing-externally',
      ],
    },
    {
      type: 'category',
      label: 'Structured queries',
      collapsed: true,
      items: [
        'how-to/structured-queries/query-events-with-aql',
        'how-to/structured-queries/tagging-best-practices',
      ],
    },
    {
      type: 'category',
      label: 'User Auth',
      collapsed: true,
      items: [
        'how-to/user-auth/set-up-user-keys',
        'how-to/user-auth/manage-authorized-users'
      ],
    },
    {
      type: 'category',
      label: 'Deploying to Production',
      collapsed: true,
      items: [
        'how-to/app-auth/compiling-actyx',
        'how-to/app-auth/generate-dev-certificate',
        'how-to/app-auth/sign-app-manifest',
        'how-to/app-auth/authenticate-with-app-manifest',
        'how-to/app-auth/generate-app-license',
      ],
    },
    {
      type: 'category',
      label: 'Swarms',
      collapsed: true,
      items: [
        'how-to/swarms/setup-swarm',
        'how-to/swarms/connect-nodes',
        'how-to/swarms/configure-announced-addresses',
      ],
    },
    {
      type: 'category',
      label: 'Packaging',
      collapsed: true,
      items: [
        'how-to/packaging/mobile-apps',
        'how-to/packaging/desktop-apps',
        'how-to/packaging/headless-apps',
      ],
    },
    {
      type: 'category',
      label: 'Operations',
      collapsed: true,
      items: [
        'how-to/operations/device-management',
        'how-to/operations/discovery-helper-node',
        'how-to/operations/log-as-json',
        'how-to/operations/disable-colored-logs',
      ],
    },
    {
      type: 'category',
      label: 'Monitoring & Debugging',
      collapsed: true,
      items: [
        'how-to/monitoring-debugging/access-logs',
        'how-to/monitoring-debugging/logging-levels',
        'how-to/monitoring-debugging/network-requirements',
        'how-to/monitoring-debugging/node-connections',
      ],
    },
    {
      type: 'category',
      label: 'Licensing',
      collapsed: true,
      items: ['how-to/licensing/license-nodes', 'how-to/licensing/license-apps'],
    },
    {
      type: 'category',
      label: 'Troubleshooting',
      collapsed: true,
      items: [
        'how-to/troubleshooting/installation-and-startup',
        'how-to/troubleshooting/app-to-node-communication',
        'how-to/troubleshooting/node-to-cli-communication',
        'how-to/troubleshooting/node-synchronization',
      ],
    },
  ],
  conceptualSidebar: [
    'conceptual/overview',
    'conceptual/how-actyx-works',
    'conceptual/event-streams',
    'conceptual/tags',
    'conceptual/actyx-jargon',
    'conceptual/discovery',
    'conceptual/performance-and-limits',
    'conceptual/authentication-and-authorization',
    'conceptual/operations',
    'conceptual/security',
    'conceptual/distributed-systems',
    'conceptual/event-sourcing',
  ],
  referenceSidebar: [
    'reference/overview',
    'reference/actyx-reference',
    {
      type: 'category',
      label: 'Actyx API',
      collapsed: true,
      items: [
        'reference/node-api',
        'reference/auth-api',
        'reference/events-api',
        'reference/files-api',
      ],
    },
    'reference/node-manager',
    {
      type: 'category',
      label: 'Actyx CLI',
      collapsed: true,
      items: [
        'reference/cli/cli-overview',
        'reference/cli/apps/sign',
        'reference/cli/events/dump',
        'reference/cli/events/offsets',
        'reference/cli/events/publish',
        'reference/cli/events/query',
        'reference/cli/events/restore',
        'reference/cli/nodes/ls',
        'reference/cli/nodes/inspect',
        'reference/cli/settings/schema',
        'reference/cli/settings/get',
        'reference/cli/settings/set',
        'reference/cli/settings/unset',
        'reference/cli/swarms/keygen',
        'reference/cli/users/keygen',
        'reference/cli/users/add-key',
        'reference/cli/topics/delete',
        'reference/cli/topics/ls',
      ],
    },
    'reference/aql',
  ],
  tutorialSidebar: [
    {
      type: 'doc',
      id: 'tutorials/overview', // string - document id
    },
    {
      type: 'category', label: 'Getting Started', collapsed: false, items: [
        'tutorials/getting-started/first-event',
        'tutorials/getting-started/first-query',
        {
          type: 'category', label: 'Machine Runner', collapsed: false, items: [
            'tutorials/getting-started/machine-runner/first-machine',
            'tutorials/getting-started/machine-runner/event-payloads',
            'tutorials/getting-started/machine-runner/state-payloads',
          ]
        }
      ]
    },
    {
      type: 'doc',
      id: 'tutorials/quickstart', // string - document id
    },
    {
      type: 'doc',
      id: 'tutorials/chat', // string - document id
    },
    { type: 'doc', id: 'tutorials/aql' },
    { type: 'doc', id: 'tutorials/ephemeral_streams' },
  ],
}
