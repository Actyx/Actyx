module.exports = {
  title: 'Actyx Developer',
  tagline: 'Documentation, guides and tools for building on the Actyx Platform',
  url: 'https://developer.actyx.com',
  baseUrl: '/',
  favicon: 'img/favicon.ico',
  organizationName: 'Actyx',
  projectName: 'Actyx',
  plugins: [
    [
      require.resolve('docusaurus-gtm-plugin'),
      {
        id: 'GTM-5PXCMFH',
      },
    ],
    [require.resolve('./src/plugins/analytics'), {}],
  ],
  themeConfig: {
    announcementBar: {
      id: 'pond-2.3.0',
      content:
        '🥁 We released version 1.1.0 of ActyxOS! Read the <a target="_blank" rel="noopener noreferrer" href="https://developer.actyx.com/blog/2020/12/11/actyxos-1-1-0-release">blog post</a> to learn more or check out <a target="_blank" rel="noopener noreferrer" href="https://developer.actyx.com/docs/os/release-notes">release notes</a> ! 🥁',
      backgroundColor: '#f5f6f7',
      textColor: '#000',
    },
    colorMode: {
      defaultMode: 'light',
      disableSwitch: true,
      respectPrefersColorScheme: false,
    },
    navbar: {
      title: '',
      logo: {
        alt: 'Actyx Developer',
        src: 'img/header.svg',
      },
      items: [
        {
          label: 'ActyxOS',
          activeBasePath: 'docs/os/',
          to: 'docs/os/general/introduction',
        },
        {
          label: 'Actyx\u00a0Pond',
          activeBasePath: 'docs/pond/',
          to: 'docs/pond/introduction',
        },
        {
          label: 'Node\u00a0Management',
          position: 'left',
          items: [
            {
              label: 'Actyx\u00a0CLI',
              to: 'docs/cli/getting-started',
            },
            {
              label: 'ActyxOS\u00a0Node\u00a0Manager',
              to: 'docs/node-manager/overview',
            },
          ],
        },
        {
          to: 'docs/learn-actyx',
          activeBasePath: 'learn-actyx',
          label: 'Learn\u00a0Actyx',
          position: 'left',
        },
        {
          to: 'blog',
          label: 'Blog',
          position: 'right',
        },
      ],
    },
    footer: {
      logo: {
        alt: 'Actyx Developer',
        src: 'img/header.svg',
        href: 'https://developer.actyx.com',
      },
      style: 'light',
      links: [
        {
          title: 'Product Documentation',
          items: [
            {
              label: 'ActyxOS',
              to: 'docs/os/general/introduction',
            },
            {
              label: 'Actyx Pond',
              to: 'docs/pond/introduction',
            },
          ],
        },
        {
          title: 'Useful Links',
          items: [
            {
              label: 'FAQ',
              to: 'docs/faq/supported-programming-languages',
            },
            {
              label: 'Blog Posts',
              to: 'blog',
            },
            {
              label: 'Downloads',
              to: 'https://downloads.actyx.com/',
            },
            {
              label: 'Discord',
              to: 'https://discord.gg/262yJhc',
            },
          ],
        },
        {
          title: 'Node Management',
          items: [
            {
              label: 'Actyx CLI',
              to: 'docs/cli/getting-started',
            },
            {
              label: 'Actyx Node Manager',
              to: 'docs/node-manager/overview',
            },
          ],
        },
        {
          title: 'Node Management',
          items: [
            {
              label: 'Actyx CLI',
              to: 'docs/cli/getting-started',
            },
            {
              label: 'Actyx Node Manager',
              to: 'docs/node-manager/overview',
            }
          ]
        },
        {
          title: 'Actyx',
          items: [
            {
              label: 'Home',
              to: 'https://www.actyx.com',
            },
            {
              label: 'Team',
              to: 'https://www.actyx.com/company/team',
            },
            {
              label: 'Career',
              to: 'https://careers.actyx.io/',
            },
            {
              label: 'Press',
              to: 'https://www.actyx.com/news',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Actyx AG`,
    },
    prism: {
      theme: require('prism-react-renderer/themes/palenight'),
      darkTheme: require('prism-react-renderer/themes/dracula'),
      additionalLanguages: ['rust', 'csharp'],
    },
    algolia: {
      apiKey: 'dee14099c148f0ca14d046428003623a',
      indexName: 'actyx_developer',
      algoliaOptions: {}, // Optional, if provided by Algolia
    },
  },
  presets: [
    [
      '@docusaurus/preset-classic',
      {
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      },
    ],
  ],
};
