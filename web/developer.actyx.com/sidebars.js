module.exports = {
  homeSidebar: [
    'home/welcome',
    'home/actyx_platform',
    'home/actyx_products',
  ],
  osSidebar: [
    {
      type: 'category',
      label: 'General',
      items: [
        'os/introduction',
        'os/design-principles',
        'os/architecture',
      ]
    },
    {
      type: 'category', label:
        'Getting Started', items: [
          'os/getting-started/installation',
          'os/getting-started/licensing',
        ]
    }, {
      type: 'category', label:
        'Guides', items: [
          'os/guides/overview',
          'os/guides/swarms',
          'os/guides/building-apps',
          'os/guides/running-apps',
          'os/guides/event-streams',
        ]
    }, {
      type: 'category', label:
        'Advanced Guides', items: [
          'os/advanced-guides/overview',
          'os/advanced-guides/actyxos-on-android',
          'os/advanced-guides/actyxos-on-docker',
          'os/advanced-guides/actyxos-on-windows',
          'os/advanced-guides/node-and-app-lifecycle',
          'os/advanced-guides/node-and-app-settings',
          'os/advanced-guides/actyxos-bootstrap-node',
          'os/advanced-guides/using-workspace-one',
          'os/advanced-guides/using-balena',
          'os/advanced-guides/using-vscode-for-schema-validation',
        ]
    }, {
      type: 'category', label:
        'Theoretical Foundation', items: [
          'os/theoretical-foundation/distributed-systems',
          'os/theoretical-foundation/event-sourcing',
          'os/theoretical-foundation/actyxos-and-cap',
        ]
    }, {
      type: 'category', label:
        'API Reference', items: [
          'os/api/overview',
          'os/api/event-service',
          'os/api/blob-service',
          'os/api/console-service',
          'os/api/node-settings-schema',
          'os/api/app-manifest-schema',
        ]
    }, {
      type: 'category', label:
        'SDKs', items: [
          'os/sdks/rust',
          'os/sdks/js-ts',
        ]
    }, {
      type: 'category', label:
        'Tools', items: [
          'os/tools/node-manager',
        ]
    },
    'os/release-notes'
  ],
  pondv1Sidebar: {
    'Versions (current: v1)': [
      { type: 'link', href: '/docs/pond/introduction', label: 'v2' },
    ],
    'Actyx Pond': [
      'pond-v1/getting-started',
      'pond-v1/programming-model',
    ],
    'Guides': [
      'pond-v1/guides/hello-world',
      'pond-v1/guides/events',
      'pond-v1/guides/local-state',
      'pond-v1/guides/subscriptions',
      'pond-v1/guides/time-travel',
      'pond-v1/guides/commands',
      'pond-v1/guides/types',
      'pond-v1/guides/snapshots',
      'pond-v1/guides/integrating-a-ui',
    ],
  },
  pondSidebar: [{
    type: 'category', label:
      'Versions (current: v2)', items: [
        { type: 'link', href: '/docs/pond-v1/getting-started', label: 'v1' },
      ]
  },
    'pond/introduction',
    'pond/getting-started',
  {
    type: 'category', label:
      'Learning the Pond in 10 steps', items: [
        'pond/guides/hello-world',
        'pond/guides/events',
        'pond/guides/local-state',
        'pond/guides/subscriptions',
        'pond/guides/typed-tags',
        'pond/guides/time-travel',
        'pond/guides/state-effects',
        'pond/guides/types',
        'pond/guides/snapshots',
        'pond/guides/integrating-a-ui',
      ]
  },
  ],
  faqSidebar: [
    'faq/supported-programming-languages',
    'faq/supported-edge-devices',
    'faq/supported-device-operating-systems',
    'faq/integrating-with-machines',
    'faq/integrating-with-software-systems',
    'faq/pre-built-actyxos-apps',
    'faq/network-requirements',
    'faq/latency-and-performance',
    'faq/number-of-devices',
    'faq/running-out-of-disk-space',
  ],
  cliSidebar: [
    'cli/getting-started',
    'cli/ax',
    {
      type: 'category', label: 'ax nodes', items: [
        'cli/nodes/nodes',
        'cli/nodes/ls',
      ]
    },
    {
      type: 'category', label: 'ax apps', items: [
        'cli/apps/apps',
        'cli/apps/ls',
        'cli/apps/validate',
        'cli/apps/package',
        'cli/apps/deploy',
        'cli/apps/undeploy',
        'cli/apps/start',
        'cli/apps/stop',
      ]
    },
    {
      type: 'category', label: 'ax settings', items: [
        'cli/settings/settings',
        'cli/settings/scopes',
        'cli/settings/schema',
        'cli/settings/get',
        'cli/settings/set',
        'cli/settings/unset',
      ]
    },
    {
      type: 'category', label: 'ax logs', items: [
        'cli/logs/logs',
        'cli/logs/tail',
      ]
    },
    {
      type: 'category', label: 'ax swarms', items: [
        'cli/swarms/swarms',
        'cli/swarms/keygen',
      ]
    },
    'cli/release-notes',
  ]
};
