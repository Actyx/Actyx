module.exports = {
  title: 'Actyx Developer',
  tagline: 'Documentation, guides and tools for building on the Actyx Platform',
  url: 'https://developer.actyx.com',
  baseUrl: '/',
  favicon: 'img/favicon.ico',
  organizationName: 'Actyx',
  projectName: 'Actyx',
  plugins: ['@docusaurus/plugin-google-analytics'],
  themeConfig: {
    announcementBar: {
      id: 'support_us',
      content:
        '⭐️ We just released v1.0.0 of ActyxOS! Check our <a target="_blank" rel="noopener noreferrer" href="https://github.com/facebook/docusaurus">Blog</a> for news! ⭐️',
        backgroundColor: '#1998ff',
        textColor: '#fff',
    },
    colorMode: {
      defaultMode: 'light',
      disableSwitch: true,
      respectPrefersColorScheme: false,
    },
    disableDarkMode: true,
    sidebarCollapsible: true,
    navbar: {
      title: '',
      logo: {
        alt: 'Actyx Developer',
        src: 'img/logo.svg',
      },
      links: [
        {
          to: 'docs/quickstart',
          activeBasePath: 'docs/quickstart',
          label: 'Quickstart',
          position: 'left',
        },
        {
          to: 'docs/os/introduction',
          activeBasePath: 'docs/os/',
          label: 'ActyxOS',
          position: 'left',
        },
        {
          to: 'docs/pond/introduction',
          activeBasePath: 'docs/pond/',
          label: 'Actyx\u00a0Pond',
          position: 'left',
        },
        {
          to: 'blog',
          label: 'Blog',
          position: 'right'
        },
        {
          to: 'https://downloads.actyx.com',
          label: 'Downloads',
          position: 'right',
        },
      ],
    },
    footer: {
      logo: {
        alt: 'Actyx Logo',
        src: 'img/logo.svg',
        href: 'www.developer.actyx.com'
      },
      style: 'light',
      links: [
        {
          title: 'Product Documentation',
          items: [
            {
              label: 'ActyxOS',
              to: 'docs/os/introduction',
            },
            {
              label: 'Actyx Pond',
              to: 'docs/pond/introduction',
            },
            {
              label: 'Actyx CLI',
              to: 'docs/cli/getting-started',
            },
            {
              label: 'Actyx Node Manager',
              to: 'docs/os/tools/node-manager',
            } 
          ]
        },
        {
          title: 'Useful Links',
          items: [
            {
              label: 'FAQ',
              to: 'docs/faq/supported-programming-languages'
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
            } 
          ]
        }
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Actyx AG`,
    },
    prism: {
      theme: require('prism-react-renderer/themes/palenight'),
      darkTheme: require('prism-react-renderer/themes/dracula'),
      additionalLanguages: ['rust'],
    },
    googleAnalytics: {
      trackingID: 'UA-102758359-7',
      // Optional fields.
      anonymizeIP: true, // Should IPs be anonymized?
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
