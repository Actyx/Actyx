module.exports = {
  startSidebar: [
    {
      type: 'category',
      label: '🚀 Starting with Actyx',
      collapsed: false,
      items: ['start/overview'],
    },
    {
      type: 'category',
      label: '💬 Explanations',
      collapsed: false,
      items: [
        'start/explanation/the-actyx-node',
        'start/explanation/node-lifecycle',
        'start/explanation/distributed-systems',        
        'start/explanation/event-sourcing',
        'start/explanation/event-ordering',
        'start/explanation/actyx-and-cap',
      ],
    },
    {
      type: 'category',
      label: '📘 How-To-Guides',
      collapsed: false,
      items: [
        'start/how-to/actyx-on-android',
        'start/how-to/actyx-on-docker',
        'start/how-to/actyx-on-linux',
        'start/how-to/actyx-on-macos',
        'start/how-to/actyx-on-windows',
      ],
    },
    {
      type: 'category',
      label: '🤓 API Reference',
      collapsed: false,
      items: [],
    },
  ],
  buildSidebar: [
    {
      type: 'category',
      label: '👨‍💻 Building Solutions',
      collapsed: false,
      items: ['build/overview'],
    },
    {
      type: 'category',
      label: '💬 Explanations',
      collapsed: false,
      items: [
        'build/explanation/local-twins',
        'build/explanation/ui-apps',
        'build/explanation/headless-apps',
      ],
    },
    {
      type: 'category',
      label: '📘 How-To-Guides',
      collapsed: false,
      items: [
        'build/how-to/actyx-sdk',
        'build/how-to/web-apps',
        'build/how-to/windows-native',
        'build/how-to/android-native',
        'build/how-to/app-licensing',
      ],
    },
    {
      type: 'category',
      label: '🤓 API Reference',
      collapsed: false,
      items: [
        'build/reference/actyx-api',
        'build/reference/js-ts-sdk',
        'build/reference/rust-sdk',
        'build/reference/app-manifest'
      ],
    },
  ],
  deploySidebar: [
    {
      type: 'category',
      label: '🏭 Deploying to Production',
      collapsed: false,
      items: ['deploy/overview'],
    },
    {
      type: 'category',
      label: '💬 Explanations',
      collapsed: false,
      items: [
        'deploy/explanation/swarms',
        'deploy/explanation/peer-discovery',
      ],
    },
    {
      type: 'category',
      label: '📘 How-To-Guides',
      collapsed: false,
      items: [
        'deploy/how-to/app-configuration',
        'deploy/how-to/node-configuration',
        'deploy/how-to/logging',
        'deploy/how-to/deployments',
        'deploy/how-to/updating',
      ],
    },
    {
      type: 'category',
      label: '🤓 API Reference',
      collapsed: false,
      items: [
        'deploy/reference/cli',
        'deploy/reference/node-manager'
      ],
    },
  ],
}
