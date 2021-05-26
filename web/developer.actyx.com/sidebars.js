module.exports = {
  howToSidebar: [
    'how-to/overview',
    {
      type: 'category',
      label: 'Local development',
      collapsed: true,
      items: [
        'how-to/local-development/install-actyx',
        'how-to/local-development/set-up-your-environment',
        'how-to/local-development/set-up-a-new-project',
        'how-to/local-development/install-cli-node-manager',
        // 'how-to/local-development/obtaining-a-development-certificate',
      ],
    },
    /* {
      type: 'category',
      label: 'Process Logic',
      collapsed: true,
      items: [
        'how-to/process-logic/publish-to-event-streams',
        'how-to/process-logic/subscribe-to-event-streams',
        'how-to/process-logic/compute-states-from-events',
        'how-to/process-logic/automate-decision-making',
        'how-to/process-logic/deal-with-network-partitions',
        'how-to/process-logic/model-processes-in-twins',
        'how-to/process-logic/transfer-twins-into-code',
      ],
    }, 
    {
      type: 'category',
      label: 'Actyx SDK',
      collapsed: true,
      items: ['how-to/sdk/placeholder'],
    },*/
    {
      type: 'category',
      label: 'Business logic',
      collapsed: true,
      items: [
        'how-to/actyx-pond/introduction',
        'how-to/actyx-pond/getting-started',
        {
          type: 'category',
          label: 'Pond in 10 Steps',
          collapsed: true,
          items: [
            'how-to/actyx-pond/guides/hello-world',
            'how-to/actyx-pond/guides/events',
            'how-to/actyx-pond/guides/local-state',
            'how-to/actyx-pond/guides/subscriptions',
            'how-to/actyx-pond/guides/typed-tags',
            'how-to/actyx-pond/guides/time-travel',
            'how-to/actyx-pond/guides/state-effects',
            'how-to/actyx-pond/guides/types',
            'how-to/actyx-pond/guides/snapshots',
            'how-to/actyx-pond/guides/integrating-a-ui',
          ],
        },
        {
          type: 'category',
          label: 'Fish Parameters',
          collapsed: true,
          items: [
            'how-to/actyx-pond/fish-parameters/on-event',
            'how-to/actyx-pond/fish-parameters/initial-state',
            'how-to/actyx-pond/fish-parameters/where',
            'how-to/actyx-pond/fish-parameters/fish-id',
            'how-to/actyx-pond/fish-parameters/deserialize-state',
            'how-to/actyx-pond/fish-parameters/is-reset',
            'how-to/actyx-pond/fish-parameters/snapshot-threshold',
          ],
        },
        {
          type: 'category',
          label: 'Pond in-Depth',
          collapsed: true,
          items: [
            'how-to/actyx-pond/in-depth/tag-type-checking',
            'how-to/actyx-pond/in-depth/eventual-consistency',
            'how-to/actyx-pond/in-depth/do-not-ignore-events',
            'how-to/actyx-pond/in-depth/cycling-states',
            'how-to/actyx-pond/in-depth/observe-all',
            'how-to/actyx-pond/in-depth/observe-one',
            'how-to/actyx-pond/in-depth/exception-handling',
          ],
        },
        'how-to/actyx-pond/pond-extensions',
      ],
    },
    {
      type: 'category',
      label: 'User Auth',
      collapsed: true,
      items: [
        'how-to/user-auth/set-up-user-keys',
        'how-to/user-auth/manage-authorized-users',
        'how-to/user-auth/get-developer-certificate',
      ],
    },
    {
      type: 'category',
      label: 'App Auth',
      collapsed: true,
      items: [
        'how-to/app-auth/sign-app-manifest',
        'how-to/app-auth/authenticate-with-app-manifest',
        'how-to/app-auth/manage-authorized-apps',
      ],
    },
    {
      type: 'category',
      label: 'Swarms',
      collapsed: true,
      items: [
        'how-to/swarms/setup-swarm',
        'how-to/swarms/setup-bootstrap-node',
        'how-to/swarms/configure-announced-addresses',
      ],
    },
    /*     {
      type: 'category',
      label: 'Data import and export',
      collapsed: true,
      items: [
        'how-to/integrate-with-actyx/user-interface',
        'how-to/integrate-with-actyx/other-software',
        'how-to/integrate-with-actyx/front-end-frameworks',
        'how-to/integrate-with-actyx/machines',
        'how-to/integrate-with-actyx/erps',
        'how-to/integrate-with-actyx/bi-analytics',
      ],
    }, */
    /*     {
      type: 'category',
      label: 'Testing',
      collapsed: true,
      items: [
        'how-to/testing/test-pipeline',
        'how-to/testing/unit-test-with-jest',
        'how-to/testing/unit-test-with-cypress',
        'how-to/testing/integration-testing',
        'how-to/testing/ci-cd-pipeline',
      ],
    }, */
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
      items: ['how-to/operations/device-management'],
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
        // 'how-to/monitoring-debugging/app-logs',
        // 'how-to/monitoring-debugging/connectivity-status',
        // 'how-to/monitoring-debugging/mobile-device-management',
        // 'how-to/monitoring-debugging/bash',
      ],
    },
    /*     {
      type: 'category',
      label: 'Common Use-Cases',
      collapsed: true,
      items: [
        'how-to/common-use-cases/show-data-on-a-dashboard',
        'how-to/common-use-cases/display-erp-orders-on-tablets',
        'how-to/common-use-cases/control-agvs',
        'how-to/common-use-cases/parameterise-assembly-tool',
      ],
    }, */
    {
      type: 'category',
      label: 'Troubleshooting',
      collapsed: true,
      items: [
        'how-to/troubleshooting/installation-and-startup',
        'how-to/troubleshooting/node-to-cli-communication',
        'how-to/troubleshooting/node-synchronization',
      ],
    },
    {
      type: 'category',
      label: 'Migration from v1',
      collapsed: true,
      items: [
        'how-to/migration/migration-overview',
        {
          type: 'category',
          label: 'Apps',
          items: [
            'how-to/migration/migrate-business-logic',
            'how-to/migration/migrate-app-manifest',
            'how-to/migration/migrate-app-logs-and-settings',
          ],
        },
      ],
    },
  ],
  conceptualSidebar: [
    'conceptual/overview',
    'conceptual/how-actyx-works',
    'conceptual/event-sourcing',
    'conceptual/distributed-systems',
    'conceptual/local-first-cooperation',
    'conceptual/actyx-jargon',
    // 'conceptual/actyx-vs-the-cloud',
    // 'conceptual/peer-discovery',
    'conceptual/performance-and-limits',
    'conceptual/authentication-and-authorization',
    'conceptual/operations',
    'conceptual/security',
    // 'conceptual/apps-in-the-factory-context',
  ],
  referenceSidebar: [
    'reference/overview',
    'reference/actyx-reference',
    {
      type: 'category',
      label: 'Actyx API',
      collapsed: true,
      items: ['reference/events-api', 'reference/auth-api'],
    },
    {
      type: 'category',
      label: 'Actyx Pond (JS/TS)',
      collapsed: true,
      items: require('./__pond-sidebar'),
    },
    'reference/node-manager',
    {
      type: 'category',
      label: 'Actyx CLI',
      collapsed: true,
      items: [
        'reference/cli/cli-overview',
        'reference/cli/nodes/ls',
        'reference/cli/settings/scopes',
        'reference/cli/settings/schema',
        'reference/cli/settings/get',
        'reference/cli/settings/set',
        'reference/cli/settings/unset',
        'reference/cli/swarms/keygen',
        'reference/cli/users/keygen',
      ],
    },
    'reference/release-notes',
  ],
  tutorialSidebar: [
    {
      type: 'doc',
      id: 'tutorials/overview', // string - document id
    },
    {
      type: 'doc',
      id: 'tutorials/quickstart', // string - document id
    },
    {
      type: 'doc',
      id: 'tutorials/chat', // string - document id
    },
    {
      type: 'category',
      label: 'Advanced Tutorial',
      collapsed: true,
      items: [
        'tutorials/advanced-tutorial/introduction',
        'tutorials/advanced-tutorial/architecture',
        'tutorials/advanced-tutorial/get-started',
        'tutorials/advanced-tutorial/project',
        'tutorials/advanced-tutorial/business-logic',
        'tutorials/advanced-tutorial/packaging',
        'tutorials/advanced-tutorial/next-steps',
      ],
    },
  ],
}
