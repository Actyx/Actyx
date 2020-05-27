module.exports = {
  osSidebar: {
    'ActyxOS': [
      'os/introduction',
      'os/design-principles',
      'os/architecture',
    ],
    'Getting Started': [
      'os/getting-started/installation',
      'os/getting-started/licensing',
    ],
    'Guides': [
      'os/guides/swarms',
      'os/guides/building-apps',
      'os/guides/running-apps',
      'os/guides/event-streams',
    ],
    'Advanced Guides': [
      'os/advanced-guides/app-runtimes',
      'os/advanced-guides/node-and-app-lifecycle',
      'os/advanced-guides/node-and-app-settings',
      'os/advanced-guides/actyxos-on-android',
      'os/advanced-guides/actyxos-on-docker',
      'os/advanced-guides/actyxos-bootstrap-node',
      'os/advanced-guides/using-workspace-one',
      'os/advanced-guides/using-balena',
      'os/advanced-guides/using-vscode-for-schema-validation',
    ],
    'Theoretical Foundation': [
      'os/theoretical-foundation/distributed-systems',
      'os/theoretical-foundation/event-sourcing',
      'os/theoretical-foundation/actyxos-and-cap',
    ],
    'API Reference': [
      'os/api/event-service',
      'os/api/blob-service',
      'os/api/console-service',
      'os/api/node-settings-schema',
      'os/api/app-manifest-schema',
    ],
    'SDKs': [
      'os/sdks/rust',
    ],
  },
  pondSidebar: {
    'Actyx Pond': [
      'pond/getting-started',
      'pond/programming-model',
    ],
    'Guides': [
      'pond/guides/hello-world',
      'pond/guides/events',
      'pond/guides/local-state',
      'pond/guides/subscriptions',
      'pond/guides/time-travel',
      'pond/guides/commands',
      'pond/guides/types',
      'pond/guides/snapshots',
      'pond/guides/integrating-a-ui',
    ],
  },
  faqSidebar: {
    'FAQs': [
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
  },
  cliSidebar: {
    'Actyx CLI': [
      'cli/getting-started',
      'cli/ax',
    ],
    'ax nodes': [
      'cli/nodes/nodes',
      'cli/nodes/ls',
    ],
    'ax apps': [
      'cli/apps/apps',
      'cli/apps/ls',
      'cli/apps/validate',
      'cli/apps/package',
      'cli/apps/deploy',
      'cli/apps/undeploy',
      'cli/apps/start',
      'cli/apps/stop',
    ],
    'ax settings': [
      'cli/settings/settings',
      'cli/settings/scopes',
      'cli/settings/schema',
      'cli/settings/get',
      'cli/settings/set',
      'cli/settings/unset',
    ],
    'ax logs': [
      'cli/logs/logs',
      'cli/logs/tail',
    ],
    'ax swarms': [
      'cli/swarms/swarms',
      'cli/swarms/keygen',
    ],
  }
};
