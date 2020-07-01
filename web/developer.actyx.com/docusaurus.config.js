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
    navbar: {
      title: 'Actyx Developers',
      logo: {
        alt: 'Actyx Developer',
        src: 'img/logo.svg',
      },
      links: [
        {
          to: 'docs/home/welcome',
          activeBasePath: 'docs/home/',
          label: 'Home',
          position: 'left',
        },
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
          to: 'docs/pond/getting-started',
          activeBasePath: 'docs/pond/',
          label: 'Actyx Pond',
          position: 'left',
        },
        {
          to: 'docs/cli/getting-started',
          activeBasePath: 'docs/cli/',
          label: 'Actyx CLI',
          position: 'left',
        },
        {
          to: 'docs/faq/supported-programming-languages',
          activeBasePath: 'docs/faq/',
          label: 'FAQ',
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
        {
          to: 'https://www.actyx.com',
          label: 'www.actyx.com',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'light',
      copyright: `Copyright © ${new Date().getFullYear()} Actyx AG`,
    },
    prism: {
      theme: require('prism-react-renderer/themes/github'),
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
