
const osDoc = (path) => `os/${path}`
const pondDoc = (path) => `pond/${path}`
const faqDoc = (path) => `faq/${path}`
module.exports = {
  osSidebar: {
    'ActyxOS': [
      'os/introduction',
      'os/design-principles',
      'os/architecture',
    ],
    'Getting Started': [
      'os/getting-started/installation',
      'os/getting-started/configuration',
    ],
    'Guides': [
      'os/guides/hello-world',
      'os/guides/event-streams',
      'os/guides/blob-storage',
      'os/guides/user-interfaces',
      'os/guides/logging',
    ],
    'Advanced Guides': [
      'os/advanced-guides/webview-runtime',
      'os/advanced-guides/docker-runtime',
      'os/advanced-guides/event-service',
      'os/advanced-guides/blob-service',
      'os/advanced-guides/console-service',
      'os/advanced-guides/node-and-app-settings',
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
  },
  pondSidebar: {
    'Actyx Pond': [
      'pond/introduction',
      'pond/design-principles',
    ],
    'Getting Started': [
      'pond/getting-started/installation',
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
  }
};
